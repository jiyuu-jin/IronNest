# roku

A blocking client for Roku TVs and players over Roku's External Control
Protocol (ECP): plain HTTP on port 8060 of the device, no account.

- `device_info` (name, model, power mode), `active_app`, `apps`;
- `keypress` (any ECP key: `Home`, `Play`, `PowerOn`, `PowerOff`, …),
  `power(on)`, `launch(app)`.

The device must have "Control by mobile apps" set to allow network access
(Settings ▸ System ▸ Advanced system settings).
