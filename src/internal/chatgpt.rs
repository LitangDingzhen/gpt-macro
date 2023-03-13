// SPDX-License-Identifier: MIT
// Akira Moroo <retrage01@gmail.com> 2023

use hyper::client::HttpConnector;
use hyper::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use hyper::{Body, Client, Request, Uri};
use hyper_proxy::{Intercept, Proxy, ProxyConnector};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use std::any::Any;
use tokio::runtime::Runtime;

use crate::internal::completion::CodeCompletion;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Role {
    User,
    System,
    Assistant,
}

#[derive(Deserialize, Serialize, Debug)]
struct ChatMessage {
    role: Role,
    content: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Chat {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatCompletion {
    id: String,
    object: String,
    created: u64,
    choices: Vec<ChatChoice>,
    usage: ChatUsage,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatChoice {
    index: u32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

pub struct ChatGPT {
    chat: Chat,
}

impl ChatGPT {
    const URL: &'static str = "https://api.openai.com/v1/chat/completions";
    const MODEL: &'static str = "gpt-3.5-turbo";

    fn add_message(&mut self, role: Role, content: String) {
        self.chat.messages.push(ChatMessage { role, content });
    }

    async fn completion(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set");
        let uri: Uri = Self::URL.parse()?;
        let https_connector = HttpsConnector::new();

        let proxy_connector = if let Ok(proxy_uri) = std::env::var("HTTP_PROXY") {
            let proxy_uri = proxy_uri.parse().unwrap();
            let proxy = Proxy::new(Intercept::All, proxy_uri);
            let proxy_connector =
                ProxyConnector::from_proxy(https_connector.clone(), proxy).unwrap();
            Some(proxy_connector)
        } else {
            None
        };
        let client = proxy_connector.map_or_else(
            || Box::new(Client::builder().build::<_, hyper::Body>(https_connector)) as Box<dyn Any>,
            |proxy| Box::new(Client::builder().build::<_, hyper::Body>(proxy)),
        );

        let body = Body::from(serde_json::to_string(&self.chat)?);

        let mut request_body = Request::new(body);

        *request_body.method_mut() = hyper::Method::POST;
        *request_body.uri_mut() = uri.clone();

        request_body
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        request_body.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
        );
        let request = move |req: Request<Body>| {
            if let Some(c) = client.downcast_ref::<Client<HttpsConnector<HttpConnector>>>() {
                c.request(req)
            } else if let Some(c) =
                client.downcast_ref::<Client<ProxyConnector<HttpsConnector<HttpConnector>>>>()
            {
                c.request(req)
            } else {
                panic!("Unknown client type");
            }
        };
        let response = request(request_body).await?;
        let body_bytes = hyper::body::to_bytes(response.into_body()).await?;
        let body_str = String::from_utf8(body_bytes.to_vec())?;

        let chat_completion: ChatCompletion = serde_json::from_str(&body_str)?;

        let content = chat_completion.choices[0].message.content.clone();

        println!("Response from ChatGPT:\n{}", content);

        self.add_message(Role::Assistant, content);

        Ok(())
    }

    fn extract_code(&self) -> Result<String, Box<dyn std::error::Error>> {
        let last_content = self.chat.messages[self.chat.messages.len() - 1]
            .content
            .clone();
        // Remove the code block and remaining explanation text.
        // Extract the test case in the code block. Other parts are removed.
        let code_block = last_content
            .split("```rust")
            .nth(1)
            .ok_or(format!("No code block start found: {}", last_content))?
            .split("```")
            .next()
            .ok_or(format!("No code block end found: {}", last_content))?
            .trim()
            .to_string();

        Ok(code_block)
    }
}

impl CodeCompletion for ChatGPT {
    fn new() -> Self {
        Self {
            chat: Chat {
                model: Self::MODEL.to_string(),
                messages: vec![],
            },
        }
    }

    fn init(&mut self, init_prompt: String) {
        self.add_message(Role::System, init_prompt);
    }

    fn add_context(&mut self, context: String) {
        self.add_message(Role::User, context);
    }

    fn code_completion(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let rt = Runtime::new()?;

        rt.block_on(self.completion())?;

        self.extract_code()
    }
}
