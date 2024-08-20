use super::{FieldContextInjection, FieldInjection, FieldValidationState};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use std::ops::Deref;

type RuleValidator<T> = Box<dyn Fn(&T, Signal<Option<String>>) -> Result<(), FieldValidationState>>;

pub struct Rule<T, Trigger> {
    validator: RuleValidator<T>,
    trigger: Trigger,
}

impl<T, Trigger> Rule<T, Trigger> {
    pub fn validator(
        f: impl Fn(&T, Signal<Option<String>>) -> Result<(), FieldValidationState>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Trigger: Default,
    {
        Self {
            trigger: Default::default(),
            validator: Box::new(f),
        }
    }

    pub fn with_trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;

        self
    }

    pub fn validate<V, R>(
        rules: Vec<R>,
        value: V,
        name: Signal<Option<String>>,
    ) -> Callback<Option<Trigger>, bool>
    where
        V: WithUntracked<Value = T>,
        V: Send + Sync + Copy + 'static,
        R: Deref<Target = Rule<T, Trigger>> + 'static,
        Trigger: PartialEq + 'static,
    {
        let field_injection = FieldInjection::use_context();
        let rules = StoredValue::new(SendWrapper::new(rules));
        let validate = Callback::new(move |trigger: Option<Trigger>| {
            let state = rules.with_value(move |rules| {
                if rules.is_empty() {
                    return Some(Ok(()));
                }

                let mut rules_iter = rules.iter();
                let mut call_count = 0;
                loop {
                    let Some(rule) = rules_iter.next() else {
                        if call_count == 0 {
                            return None;
                        } else {
                            return Some(Ok(()));
                        }
                    };

                    if let Some(trigger) = trigger.as_ref() {
                        if &rule.trigger != trigger {
                            continue;
                        }
                    }
                    call_count += 1;

                    let state = value.with_untracked(|value| (rule.validator)(value, name));
                    if state.is_err() {
                        return Some(state);
                    }
                }
            });

            let Some(state) = state else {
                return true;
            };

            let rt = state.is_ok();
            if let Some(field_injection) = field_injection.as_ref() {
                field_injection.update_validation_state(state);
            };
            rt
        });
        if let Some(field_context) = FieldContextInjection::use_context() {
            field_context.register_field(name, move || validate.run(None));
        }

        validate
    }
}
