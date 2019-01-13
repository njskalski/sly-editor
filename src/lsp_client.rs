/*
Copyright 2018 Google LLC

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

// Some code in this file is copied from https://github.com/hatoo/Accepted/blob/master/src/lsp.rs ver eba4846
// or at least heavily inspired with. Thanks for figuring out how to work with languageserver_types
// crate.

use languageserver_types as lst;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::process::{Child, Command};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

use events::IChannel;
use events::IEvent;
use jsonrpc_core::types as jt;
use jsonrpc_core::Output;
use languageserver_types;

pub struct LspClient {
    waiter_handle :  JoinHandle<()>,
    is_initialized : bool,
    i_event_sink :   IChannel,
    channel :        (Sender<LSPEvent>, Receiver<LSPEvent>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LSPEvent {
    Initialized,
}

const ID_INIT : u64 = 0; // it's always a first message.
const ID_COMPLETION : u64 = 0;

impl LspClient {
    pub fn new(
        path_to_program : &OsStr,
        event_sink : IChannel,
        workspace_folders : Option<&Vec<PathBuf>>,
    ) -> Result<LspClient, Box<Error>> {
        let workspace_folders_op = workspace_folders.map(|v| {
            v.iter()
                .map(|path| {
                    let uri = path.to_str().unwrap().to_owned();
                    let name = uri.clone();
                    lst::WorkspaceFolder { uri, name }
                })
                .collect::<Vec<lst::WorkspaceFolder>>()
        });

        let init = languageserver_types::InitializeParams {
            process_id :             Some(u64::from(process::id())),
            root_path :              Some("./".to_string()),
            root_uri :               None,
            initialization_options : None,
            capabilities :           languageserver_types::ClientCapabilities::default(),
            trace :                  Some(lst::TraceOption::Verbose), /* TODO(njskalski) */
            workspace_folders :      workspace_folders_op,
        };

        let mut lsp_command = Command::new(path_to_program);

        debug!("starting LSP");

        let mut lsp = lsp_command
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()?;

        debug!("started LSP");

        let mut stdin = lsp.stdin.take().ok_or("unable to grab stdin of language server")?;
        let mut reader =
            BufReader::new(lsp.stdout.take().ok_or("unable to grab stdout of language server")?);

        send_request::<_, languageserver_types::request::Initialize>(&mut stdin, ID_INIT, init)?;

        let lsp_channel = channel::<LSPEvent>();
        let lsp_sink = lsp_channel.0.clone();

        let handle = thread::spawn(move || {
            let mut headers : HashMap<String, String> = HashMap::new();
            loop {
                headers.clear();
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).unwrap() == 0 {
                        return;
                    }
                    let header = header.trim();
                    if header.is_empty() {
                        break;
                    }
                    let parts : Vec<&str> = header.split(": ").collect();
                    if parts.len() != 2 {
                        return;
                    }
                    headers.insert(parts[0].to_string(), parts[1].to_string());
                }
                let content_len = headers["Content-Length"].parse().unwrap();
                let mut content = vec![0; content_len];
                reader.read_exact(&mut content).unwrap();

                let msg = String::from_utf8(content).unwrap();

                let output : serde_json::Result<Output> = serde_json::from_str(&msg);
                debug!("dd : {:?}", &output);
                match output {
                    Ok(jt::Output::Success(suc)) => {
                        if suc.id == jsonrpc_core::id::Id::Num(ID_INIT) {
                            lsp_sink.send(LSPEvent::Initialized).unwrap();
                        } else if suc.id == jsonrpc_core::id::Id::Num(ID_COMPLETION) {
                            let completion = serde_json::from_value::<
                                languageserver_types::CompletionResponse,
                            >(suc.result)
                            .unwrap();

                            //                        let mut completion =
                            // extract_completion(completion);
                            //                        tx.send(completion).unwrap();
                        }
                    }
                    Ok(jt::Output::Failure(f)) => {
                        debug!("lsp: unable to parse \n{}\nfailure:\n{:?}\n", msg, f)
                    }
                    Err(e) => debug!("lsp: unable to parse \n{}\nerrror:\n{:?}\n", msg, e),
                };
            }
        });

        Ok(LspClient {
            waiter_handle :  handle,
            is_initialized : false,
            i_event_sink :   event_sink,
            channel :        lsp_channel,
        })
    }
}

fn send_request<T : Write, R : lst::request::Request>(
    write : &mut T,
    id : u64,
    params : R::Params,
) -> Result<(), io::Error>
where
    R::Params : serde::Serialize,
{
    if let serde_json::value::Value::Object(params) = serde_json::to_value(params).unwrap() {
        let req = jsonrpc_core::Call::MethodCall(jsonrpc_core::MethodCall {
            jsonrpc : Some(jsonrpc_core::Version::V2),
            method :  R::METHOD.to_string(),
            params :  jsonrpc_core::Params::Map(params),
            id :      jsonrpc_core::Id::Num(id),
        });
        debug!("sending {:?}", &req);
        let request = serde_json::to_string(&req).unwrap();
        write!(write, "Content-Length: {}\r\n\r\n{}", request.len(), request)
    } else {
        Ok(())
    }
}

fn send_notify<T : Write, R : languageserver_types::notification::Notification>(
    write : &mut T,
    params : R::Params,
) -> Result<(), io::Error>
where
    R::Params : serde::Serialize,
{
    if let serde_json::value::Value::Object(params) = serde_json::to_value(params).unwrap() {
        let req = jsonrpc_core::Notification {
            jsonrpc : Some(jsonrpc_core::Version::V2),
            method :  R::METHOD.to_string(),
            params :  jsonrpc_core::Params::Map(params),
        };
        let request = serde_json::to_string(&req).unwrap();
        write!(write, "Content-Length: {}\r\n\r\n{}", request.len(), request)
    } else {
        Ok(())
    }
}
