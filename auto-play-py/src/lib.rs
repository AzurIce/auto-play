use ::auto_play::FooStruct;
use pyo3::prelude::*;
use std::ffi::CString;

// A Python wrapper for FooStruct that holds a raw pointer to the actual data.
#[pyclass(unsendable)]
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

/// A Rust function that executes Python code to mutate a `FooStruct`.
pub fn mutate_foo_struct(foo: &mut FooStruct, py_code: impl AsRef<str>) -> PyResult<()> {
    let code = CString::new(py_code.as_ref())?;

    Python::with_gil(|py| {
        let py_foo = PyFooStruct {
            inner: foo as *mut _,
        };
        let py_cell = Py::new(py, py_foo)?;

        let locals = pyo3::types::PyDict::new(py);
        locals.set_item("foo", py_cell)?;

        py.run(&code, None, Some(&locals))?;

        Ok(())
    })
}

#[pyo3::pymodule]
mod _auto_play {
    use pyo3::prelude::*;

    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok((a + b).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auto_play::FooStruct;

    #[test]
    fn test_mutate_foo_struct() {
        // Suppress warning by using the recommended initialization for tests
        // In recent PyO3, we should append to inittab BEFORE initialization.
        pyo3::append_to_inittab!(_auto_play);
        Python::initialize();

        let mut foo = FooStruct::new();
        println!("{foo:?}");
        assert_eq!(foo.get_count(), 0);

        let py_code = r#"
foo.increment()
foo.add(10)
"#;

        mutate_foo_struct(&mut foo, py_code).expect("Python execution failed");

        println!("{foo:?}");
        assert_eq!(foo.get_count(), 11);
    }
}
