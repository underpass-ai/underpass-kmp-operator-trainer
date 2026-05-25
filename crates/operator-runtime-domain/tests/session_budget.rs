use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::error::runtime_domain_error::RuntimeDomainError;

#[test]
fn consume_call_decrements_without_touching_tokens() {
    let budget = SessionBudget::new(2, 4096);

    let next = budget.try_consume_call().expect("one call is available");

    assert_eq!(next.calls_remaining(), 1);
    assert_eq!(next.tokens_remaining(), 4096);
    assert!(next.allows_call());
}

#[test]
fn consume_call_returns_err_when_empty() {
    let budget = SessionBudget::new(0, 4096);

    let err = budget.try_consume_call().expect_err("budget is empty");

    assert_eq!(err, RuntimeDomainError::BudgetExhausted);
    assert!(!budget.allows_call());
}
