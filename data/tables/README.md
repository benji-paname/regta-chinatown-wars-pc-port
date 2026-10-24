# Private reference tables

Only this README is tracked. Do not commit any `.bin` files here.

The original local installation contains ten tables moved byte-for-byte from `outputs/CTW-Rust/data/tables/`. They match the supplied **Android** reference port, not a verified PSP extraction. The runtime's optional sound/weapon/radio/weather table readers may use them. Their absence in a public checkout is expected; unit tests use generated in-memory fixtures instead.

See [status and provenance](../../docs/STATUS.md). These files are research inputs, not distributable assets or evidence of PSP parity. No automatic download or Android-to-PSP conversion is provided.
