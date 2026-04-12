#[derive(Debug, Clone)]
pub struct DoctorResult {
    pub summary: String,
}

pub fn doctor() -> DoctorResult {
    DoctorResult {
        summary: "doctor is not implemented yet; tool adapters will live in shosei-core::toolchain"
            .to_string(),
    }
}
