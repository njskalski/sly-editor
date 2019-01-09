use languageserver_types::request::Request;
use languageserver_types::*;
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::process::{Child, Command};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::*;

#[derive(Clone)]
enum LspWaiterMsg {}

struct LspWaiter {
    command: OsString,
    lsp_process: Child,
    child_thread: JoinHandle<()>,
    //    channel: (Sender<LspWaiterMsg>, Receiver<LspWaiterMsg>),
}

impl LspWaiter {
    //
    //    fn send<R>()
    //    where
    //        R: Request,
    //        R::Params: serde::Serialize,
    //        R::Result: serde::de::DeserializeOwned,
    //    {
    //        self.ls
    //    }

    pub fn initialize(&self, workspace_folders: Option<Vec<PathBuf>>) {
        let workspace_folders_op = workspace_folders.map(|ref v| {
            v.iter()
                .map(|path| {
                    let uri = path.to_str().unwrap().to_owned();
                    let name = uri.clone();
                    WorkspaceFolder { uri, name }
                })
                .collect::<Vec<WorkspaceFolder>>()
        });

        let init = languageserver_types::InitializeParams {
            process_id: Some(u64::from(process::id())),
            root_path: Some("./".to_string()),
            root_uri: None,
            initialization_options: None,
            capabilities: languageserver_types::ClientCapabilities::default(),
            trace: Some(TraceOption::Verbose), //TODO(njskalski)
            workspace_folders: workspace_folders_op,
        };
    }

    pub fn start(&mut self) {
        self.lsp_process = match Command::new(&self.command).spawn() {
            Ok(child) => child,
            Err(e) => {
                error!("unable to start Language Server \"{:?}\" : {:}", self.command, e);
                return;
            }
        };

        //        self.channel = channel();

        // variables below will be moved into child thread context.
        //        let (chout, chin) = (self.channel.0.clone(), self.channel.1.clone());
        //        let (stdin, stdout, stderr) : (Write, Read, Read) = self.lsp_process.map(|child| {
        //            (child.stdin.unwrap(), child.stdout.unwrap(), child.stderr.unwrap()) } );

        self.child_thread = thread::spawn(move || {

            //            stdin.write(lsp::request::Initialize{});

            //            lsp_request!("initialize");
        });
    }
}
