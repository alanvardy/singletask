//! Responses are for generating JSON and mocking API calls

/// File name is the same as the enum name
/// So you can find the `Task` variant in tests/responses/Task.json
#[allow(dead_code)]
#[derive(strum_macros::Display)]
pub enum ResponseFromFile {
    Tasks,
    Sync,
}

#[allow(dead_code)]
impl ResponseFromFile {
    /// Loads JSON responses from file for testing
    pub async fn read(&self) -> String {
        let path = format!("tests/{self}.json");

        std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Could not find json file at {path}"))
    }
}
