use auto_play::{AutoPlay, FooStruct, MatcherOptions};
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyBytes;
use std::ffi::CString;
use std::io::Cursor;

// --- PyFooStruct (Demo) ---

#[pyclass(unsendable, name = "FooStruct")]
struct PyFooStruct {
    inner: *mut FooStruct,
}

#[pymethods]
impl PyFooStruct {
    fn increment(&mut self) {
        unsafe {
            if let Some(foo) = self.inner.as_mut() {
                foo.increment();
            }
        }
    }

    fn add(&mut self, value: i32) {
        unsafe {
            if let Some(foo) = self.inner.as_mut() {
                foo.add(value);
            }
        }
    }
    
    fn get_count(&self) -> i32 {
         unsafe {
            if let Some(foo) = self.inner.as_ref() {
                foo.get_count()
            } else {
                0
            }
        }
    }
}

pub fn mutate_foo_struct(foo: &mut FooStruct, py_code: impl AsRef<str>) -> PyResult<()> {
    let code = CString::new(py_code.as_ref())?;
    
    Python::with_gil(|py| {
        let py_foo = PyFooStruct { inner: foo as *mut _ };
        let py_cell = Py::new(py, py_foo)?;
        
        let locals = pyo3::types::PyDict::new(py);
        locals.set_item("foo", py_cell)?;

        py.run(&code, None, Some(&locals))?;
        
        Ok(())
    })
}

// --- PyAutoPlay (Core Binding) ---

#[pyclass(name = "AutoPlay")]
struct PyAutoPlay {
    inner: AutoPlay,
}

#[pymethods]
impl PyAutoPlay {
    /// Connect to a device.
    #[staticmethod]
    fn connect(serial: &str) -> PyResult<Self> {
        let inner = AutoPlay::connect(serial)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to connect: {}", e)))?;
        Ok(Self { inner })
    }

    /// Check if screen is on.
    fn is_screen_on(&self) -> PyResult<bool> {
        self.inner.is_screen_on()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Ensure screen is on.
    fn ensure_screen_on(&self) -> PyResult<()> {
        self.inner.ensure_screen_on()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get device ABI.
    fn get_abi(&self) -> PyResult<String> {
        self.inner.get_abi()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get SDK version.
    fn get_sdk(&self) -> PyResult<String> {
        self.inner.get_sdk()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Click at coordinates.
    fn click(&self, x: u32, y: u32) -> PyResult<()> {
        self.inner.click(x, y)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Capture screen and return as PNG bytes.
    fn screencap<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let img = self.inner.screencap()
            .map_err(|e| PyRuntimeError::new_err(format!("Screencap failed: {}", e)))?;
        
        let mut bytes: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
             .map_err(|e| PyRuntimeError::new_err(format!("Image encoding failed: {}", e)))?;

        Ok(PyBytes::new(py, &bytes))
    }
    
    // TODO: Add find_image and other complex methods later
}

// --- Module Definition ---

#[pyo3::pymodule]
mod _auto_play {
    use super::*;
    use pyo3::prelude::*;

    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok((a + b).to_string())
    }

    #[pymodule_export]
    use super::PyAutoPlay;
}