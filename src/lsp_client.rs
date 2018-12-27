use std::process::{Child, Command};
use std::ffi::{OsStr, OsString};
use std::thread;
use std::thread::*;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::io::Write;
use std::io::Read;
use languageserver_types::*;
use languageserver_types::request::Request;


#[derive(Clone)]
enum LspWaiterMsg {

}

struct LspWaiter {
    command : OsString,
    lsp_process: Child,
    child_thread : JoinHandle<()>,
//    channel: (Sender<LspWaiterMsg>, Receiver<LspWaiterMsg>),
}

impl LspWaiter {

    fn send<R>()
    where
        R: Request,
        R::Params: serde::Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        self.ls
    }

    pub fn initialize(&self) {
        serde_json::to_writer(self.lsp_process.stdin.unwrap(), request::Initialize);
//        self.lsp_process.stdin.unwrap().write(
//            lsp_request!("initialize")
//        );
    }

    pub fn start(&mut self) {
        self.lsp_process = match Command::new(self.command).spawn() {
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