```
➜  ~ adb devices
List of devices attached
10AECG3DJN002LD device
```

```
➜  ~ adb devices -l
List of devices attached
10AECG3DJN002LD        device usb:2-1 product:PD2405 model:V2405DA device:PD2405 transport_id:1
```

## Controller

Controller 是对设备操作的第一层封装。不同的平台会有不一样的操作（比如 Windows 会有 scroll、Android 会有 launch_app 等），但是也会有一部分共用的操作（共用的 Controller Trait）。

## AutoPlay

AutoPlay 可以由 Controller 构造，提供了比 Controller 更多的功能，比如加载任务、加载用于匹配的模板图、加载导航图等资源之类的。所以它是带泛型的 `AutoPlay<T>`。

## Action

Action 是 AutoPlay 所支持的最基本的动作，如点击、滑动等。实际执行 Action 的是 `AutoPlay<T>` 因此 Action 的实现 Trait 也需要有泛型。
