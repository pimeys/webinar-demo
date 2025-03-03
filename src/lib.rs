use std::{cell::RefCell, rc::Rc, time::Duration};

use grafbase_sdk::{
    Error, Extension, Resolver, ResolverExtension, SharedContext, Subscription,
    jq_selection::JqSelection,
    types::{Configuration, FieldDefinitionDirective, FieldInputs, FieldOutput, SchemaDirective},
};
use rand::{Rng, rngs::ThreadRng};
use uuid::Uuid;

#[derive(ResolverExtension)]
struct Echo {
    jq: Rc<RefCell<JqSelection>>,
}

#[derive(serde::Deserialize)]
struct EchoDirective {
    input: Input,
}

#[derive(serde::Deserialize)]
struct Input {
    input: serde_json::Value,
}

struct RandomBankEvents {
    jq: Rc<RefCell<JqSelection>>,
    rng: ThreadRng,
    selection: String,
}

#[derive(serde::Serialize)]
struct BankEvent {
    credit: String,
    debit: String,
    amount: i32,
}

#[derive(serde::Deserialize)]
struct BankDirective {
    selection: String,
}

impl Extension for Echo {
    fn new(_: Vec<SchemaDirective>, _: Configuration) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            jq: Rc::new(RefCell::new(JqSelection::new())),
        })
    }
}

impl Subscription for RandomBankEvents {
    fn next(&mut self) -> Result<Option<FieldOutput>, Error> {
        std::thread::sleep(Duration::from_millis(500));

        let amount = self.rng.random_range(1..=2000);

        let credit = Uuid::new_v4().to_string();
        let debit = Uuid::new_v4().to_string();

        let event = BankEvent {
            credit,
            debit,
            amount,
        };

        let value = serde_json::to_value(event).unwrap();
        let mut jq = self.jq.borrow_mut();

        let selected = jq
            .select(&self.selection, value)
            .map_err(|err| format!("Failed to filter events: {err}"))?;

        let mut output = FieldOutput::new();

        for value in selected {
            match value {
                Ok(value) => {
                    output.push_value(value);
                }
                Err(error) => {
                    output.push_error(format!("Failed to process event: {error}"));
                }
            }
        }

        Ok(Some(output))
    }
}

impl Resolver for Echo {
    fn resolve_field(
        &mut self,
        _: SharedContext,
        _: &str,
        directive: FieldDefinitionDirective<'_>,
        _: FieldInputs,
    ) -> Result<FieldOutput, Error> {
        let directive: EchoDirective = directive.arguments()?;
        let mut output = FieldOutput::new();

        output.push_value(directive.input.input);

        Ok(output)
    }

    fn resolve_subscription(
        &mut self,
        _: SharedContext,
        _: &str,
        directive: FieldDefinitionDirective<'_>,
    ) -> Result<Box<dyn Subscription>, Error> {
        let directive: BankDirective = directive.arguments()?;

        let subscription = RandomBankEvents {
            jq: self.jq.clone(),
            rng: rand::rng(),
            selection: directive.selection,
        };

        Ok(Box::new(subscription))
    }
}
