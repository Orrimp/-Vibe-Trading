//! T1911 acceptance — `pricing.rs` integration test.
//!
//! Acceptance criteria from `spec/v2-llm-strategy/tasks.md`:
//!
//! - (a) every `(provider, model)` named in the v2 default TOML
//!   resolves to a `Some` rate.
//! - (b) typo'd model id `"claude-opus-4.7"` returns `None` from
//!   `base_rate` and `LlmError::Provider` from `resolve_rate`.
//! - (c) TOML override for an existing pair shadows the base table.
//! - (d) Ollama zeros are exact `Decimal::ZERO`.

use std::collections::HashMap;

use llm::pricing::{OverrideMap, PricePerMillionTokens, base_rate, resolve_rate};
use llm::{LlmError, ProviderKind};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn t1911_a_v2_default_models_all_resolve() {
    // Models listed in feature.md § Q7 strawman + R12.1 default TOML.
    let combos = [
        (ProviderKind::Anthropic, "claude-opus-4-7"),
        (ProviderKind::Anthropic, "claude-haiku-4-5-20251001"),
        (ProviderKind::OpenAi, "gpt-5"),
        (ProviderKind::OpenAi, "gpt-5-mini"),
        (ProviderKind::Other("ollama".to_string()), "llama3"),
    ];
    for (p, m) in combos {
        let rate = base_rate(&p, m);
        assert!(
            rate.is_some(),
            "default-TOML pair ({p:?}, {m}) must have a base rate"
        );
    }
}

#[test]
fn t1911_b_typo_model_id_errors_cleanly() {
    let bad = "claude-opus-4.7";
    assert!(base_rate(&ProviderKind::Anthropic, bad).is_none());

    let err = resolve_rate(&OverrideMap::new(), &ProviderKind::Anthropic, bad)
        .expect_err("typo'd model id must error");
    match err {
        LlmError::Provider { provider, message } => {
            assert!(matches!(provider, ProviderKind::Anthropic));
            assert!(
                message.contains(bad),
                "error message must name the bad model: got {message}"
            );
        }
        other => panic!("expected LlmError::Provider, got {other:?}"),
    }
}

#[test]
fn t1911_c_override_shadows_base() {
    let mut overrides: OverrideMap = HashMap::new();
    let mut per_provider: HashMap<String, PricePerMillionTokens> = HashMap::new();
    per_provider.insert(
        "claude-opus-4-7".to_string(),
        PricePerMillionTokens {
            input_usd: dec!(7.77),
            output_usd: dec!(77.77),
            cached_input_usd: dec!(0.77),
        },
    );
    overrides.insert("anthropic".to_string(), per_provider);

    let rate = resolve_rate(&overrides, &ProviderKind::Anthropic, "claude-opus-4-7")
        .expect("override resolves");
    assert_eq!(rate.input_usd, dec!(7.77));
    assert_eq!(rate.output_usd, dec!(77.77));
    assert_eq!(rate.cached_input_usd, dec!(0.77));

    // Sanity: with no override the base rate ($15/$75/$1.50) returns.
    let base = resolve_rate(
        &OverrideMap::new(),
        &ProviderKind::Anthropic,
        "claude-opus-4-7",
    )
    .expect("base rate resolves");
    assert_eq!(base.input_usd, dec!(15.00));
}

#[test]
fn t1911_d_ollama_zero_rate_is_exact_decimal_zero() {
    let r = base_rate(&ProviderKind::Other("ollama".to_string()), "anything").unwrap();
    assert_eq!(r.input_usd, Decimal::ZERO);
    assert_eq!(r.output_usd, Decimal::ZERO);
    assert_eq!(r.cached_input_usd, Decimal::ZERO);
}
