use crate::domain::{ReviewUsage, ReviewUsageCost};
use agent_client_protocol::schema::v1::{
    SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOptions, Usage, UsageUpdate,
};

pub(super) fn context_usage(update: &UsageUpdate) -> ReviewUsage {
    ReviewUsage {
        context_used: Some(update.used),
        context_size: Some(update.size),
        cost: update.cost.as_ref().and_then(|cost| {
            (cost.amount.is_finite()
                && cost.amount >= 0.0
                && cost.currency.len() == 3
                && cost.currency.bytes().all(|byte| byte.is_ascii_alphabetic()))
            .then(|| ReviewUsageCost {
                amount: cost.amount,
                currency: cost.currency.to_ascii_uppercase(),
            })
        }),
        ..Default::default()
    }
}

pub(super) fn token_usage(usage: &Usage) -> ReviewUsage {
    ReviewUsage {
        total_tokens: Some(usage.total_tokens),
        input_tokens: Some(usage.input_tokens),
        output_tokens: Some(usage.output_tokens),
        cached_read_tokens: usage.cached_read_tokens,
        cached_write_tokens: usage.cached_write_tokens,
        thought_tokens: usage.thought_tokens,
        ..Default::default()
    }
}

/// Use the effective harness configuration, including its default, rather than the requested model.
pub(super) fn model_usage(options: &[SessionConfigOption]) -> Option<ReviewUsage> {
    let option = options.iter().find(|option| {
        option.category == Some(SessionConfigOptionCategory::Model)
            || option.id.to_string() == "model"
    })?;
    let SessionConfigKind::Select(select) = &option.kind else {
        return None;
    };
    let selected = match &select.options {
        SessionConfigSelectOptions::Ungrouped(values) => values
            .iter()
            .find(|value| value.value == select.current_value),
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| &group.options)
            .find(|value| value.value == select.current_value),
        _ => None,
    };
    Some(ReviewUsage {
        model: Some(
            selected
                .map(|value| value.name.clone())
                .unwrap_or_else(|| select.current_value.to_string()),
        ),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{Cost, SessionConfigSelectOption};

    #[test]
    fn context_is_not_total_tokens_and_absent_cost_is_unknown() {
        let usage = context_usage(&UsageUpdate::new(42, 100));
        assert_eq!(usage.context_used, Some(42));
        assert!(usage.total_tokens.is_none());
        assert!(usage.cost.is_none());
    }

    #[test]
    fn accepts_zero_cost_and_rejects_invalid_cost() {
        let usage = context_usage(&UsageUpdate::new(0, 0).cost(Cost::new(0.0, "usd")));
        assert_eq!(usage.cost.unwrap().currency, "USD");
        for amount in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(
                context_usage(&UsageUpdate::new(0, 0).cost(Cost::new(amount, "USD")))
                    .cost
                    .is_none()
            );
        }
        assert!(
            context_usage(&UsageUpdate::new(0, 0).cost(Cost::new(1.0, "?")))
                .cost
                .is_none()
        );
    }

    #[test]
    fn preserves_reported_totals_without_double_counting_cache_or_reasoning() {
        let usage = token_usage(
            &Usage::new(110, 60, 10)
                .cached_read_tokens(40)
                .thought_tokens(5),
        );
        assert_eq!(usage.total_tokens, Some(110));
        assert_eq!(usage.input_tokens, Some(60));
        assert_eq!(usage.thought_tokens, Some(5));
        assert!(usage.cached_write_tokens.is_none());
    }

    #[test]
    fn model_comes_from_effective_config() {
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "chosen",
            vec![SessionConfigSelectOption::new("chosen", "Chosen Model")],
        );
        assert_eq!(
            model_usage(&[option]).unwrap().model.as_deref(),
            Some("Chosen Model")
        );
        assert!(model_usage(&[]).is_none());
    }
}
