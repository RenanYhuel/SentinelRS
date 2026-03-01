#[cfg(test)]
mod tests {
    use crate::output::OutputMode;

    #[test]
    fn version_output_modes_defined() {
        assert_ne!(OutputMode::Human, OutputMode::Json);
    }
}
