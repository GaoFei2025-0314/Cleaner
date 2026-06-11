use c_drive_cleaner::models::{CleanupAction, RiskLevel, SourceCategory};
use c_drive_cleaner::rules::{builtin_rules, RuleScope};

#[test]
fn builtin_rules_include_user_temp_as_recommended() {
    let rules = builtin_rules();
    let user_temp = rules
        .iter()
        .find(|rule| rule.id == "user-temp")
        .expect("user-temp rule");

    assert_eq!(user_temp.title, "用户临时文件");
    assert_eq!(user_temp.risk_level, RiskLevel::Recommended);
    assert_eq!(user_temp.cleanup_action, CleanupAction::DirectDelete);
    assert_eq!(user_temp.source_category, SourceCategory::System);
    assert!(matches!(user_temp.scope, RuleScope::UserLocalAppDataRelative(_)));
}

#[test]
fn builtin_rules_never_default_select_high_risk_items() {
    for rule in builtin_rules() {
        if rule.risk_level == RiskLevel::HighRisk {
            assert!(
                !rule.default_selected,
                "high-risk rule selected by default: {}",
                rule.id
            );
        }
    }
}

#[test]
fn not_cleanable_rules_are_not_default_selected() {
    for rule in builtin_rules() {
        if rule.risk_level == RiskLevel::NotCleanable {
            assert!(
                !rule.default_selected,
                "not-cleanable rule selected by default: {}",
                rule.id
            );
        }
    }
}

#[test]
fn admin_required_rules_are_not_default_selected_in_v01() {
    for rule in builtin_rules() {
        if rule.cleanup_action == CleanupAction::RequiresAdmin {
            assert!(
                !rule.default_selected,
                "admin-required rule selected by default: {}",
                rule.id
            );
        }
    }
}

#[test]
fn chat_roots_are_explain_only_not_cleanable() {
    let rules = builtin_rules();
    for rule_id in ["wechat-data-root", "qq-data-root"] {
        let rule = rules
            .iter()
            .find(|rule| rule.id == rule_id)
            .unwrap_or_else(|| panic!("{rule_id} rule"));

        assert_eq!(rule.risk_level, RiskLevel::NotCleanable);
        assert_eq!(rule.cleanup_action, CleanupAction::ExplainOnly);
        assert!(!rule.default_selected);
    }
}
