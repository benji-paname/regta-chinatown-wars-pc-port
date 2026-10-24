// SPDX-License-Identifier: GPL-3.0-only
//! Optional private integration fixtures. Unit tests always run on synthetic data.
//! Setting CTW_TEST_REFERENCE_TABLES explicitly requires every requested file.
pub fn reference_table(name: &str) -> Option<Vec<u8>> {
    std::env::var_os("CTW_TEST_REFERENCE_TABLES").map(|root| {
        std::fs::read(std::path::PathBuf::from(root).join(name))
            .expect("Explicit private reference-table directory is incomplete or unreadable")
    })
}
