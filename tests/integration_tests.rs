use std::path::PathBuf;

use futures_util::StreamExt;
use grafbase_sdk::test::{DynamicSchema, TestConfig, TestRunner};
use indoc::indoc;
use serde::Deserialize;

#[tokio::test]
async fn test_echo() {
    let subgraph = indoc! {r#"
        extend schema
            @link(
                url: "file:///home/pimeys/code/grafbase/webinar/echo/build"
                import: ["@echoField"]
            )

        scalar JSON

        type Query {
            echo(input: JSON!): JSON @echoField
        }
    "#};

    let subgraph = DynamicSchema::builder(subgraph)
        .into_extension_only_subgraph("echo", &PathBuf::from("./build"))
        .unwrap();

    let config = TestConfig::builder()
        .with_subgraph(subgraph)
        .build("")
        .unwrap();

    let runner = TestRunner::new(config).await.unwrap();

    let result: serde_json::Value = runner
        .graphql_query(r#"query { echo(input: "hello") }"#)
        .send()
        .await
        .unwrap();

    insta::assert_json_snapshot!(result, @r#"
    {
      "data": {
        "echo": "hello"
      }
    }
    "#);
}

#[tokio::test]
async fn test_subscription() {
    let subgraph = indoc! {r#"
        extend schema
            @link(
                url: "file:///home/pimeys/code/grafbase/webinar/echo/build"
                import: ["@echoField", "@randomBankEvents"]
            )

        scalar JSON

        type BankEvent {
            credit: String!
            debit: String!
            amount: Int!
        }

        type Query {
            echo(input: JSON!): JSON @echoField
        }

        type Subscription {
            bankEvents(minimumAmount: Int!): BankEvent! @randomBankEvents(
                selection: "select(.amount >= {{args.minimumAmount}}) | ."
            )
        }
    "#};

    let subgraph = DynamicSchema::builder(subgraph)
        .into_extension_only_subgraph("echo", &PathBuf::from("./build"))
        .unwrap();

    let config = TestConfig::builder()
        .with_subgraph(subgraph)
        .build("")
        .unwrap();

    let runner = TestRunner::new(config).await.unwrap();

    let query = indoc! {r#"
        subscription {
            bankEvents(minimumAmount: 1000) {
                credit
                debit
                amount
            }
        }
    "#};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Event {
        data: Data,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Data {
        bank_events: BankEvent,
    }

    #[derive(Deserialize)]
    #[allow(unused)]
    #[serde(rename_all = "camelCase")]
    struct BankEvent {
        credit: String,
        debit: String,
        amount: i32,
    }

    let subscription = runner
        .graphql_subscription::<Event>(query)
        .unwrap()
        .subscribe()
        .await
        .unwrap();

    let events = subscription.take(3).collect::<Vec<_>>().await;
    assert_eq!(3, events.len());

    assert!(events[0].data.bank_events.amount >= 1000);
    assert!(events[1].data.bank_events.amount >= 1000);
    assert!(events[2].data.bank_events.amount >= 1000);
}
