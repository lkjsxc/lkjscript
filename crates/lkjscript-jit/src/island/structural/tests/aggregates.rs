use super::{aggregate_nested::result_error_tree, aggregate_text::*};

#[test]
fn generated_option_result_and_nested_aggregates_execute_directly(
) -> Result<(), Box<dyn std::error::Error>> {
    option_string()?;
    result_path()?;
    result_error_tree()?;
    Ok(())
}
