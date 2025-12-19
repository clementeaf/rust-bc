/// Trait for API gateway implementations
pub trait ApiGateway {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_gateway_trait() {
        // Trait definition compiles
        assert!(true);
    }
}
