#![allow(unused)]

use std::sync::{Arc, Mutex, RwLock};

use argparse::{ArgumentParser, StoreOption};
use riker::actors::*;

#[derive(Debug)]
struct ActorCreation {
    parent: Option<BasicActorRef>,
    r1: usize,
    spread: usize,
    receiving: bool,
}

impl Default for ActorCreation {
    fn default() -> Self {
        ActorCreation {
            parent: None,
            r1: 0,
            spread: 0,
            receiving: false,
        }
    }
}

impl ActorFactoryArgs<BasicActorRef> for ActorCreation {
    fn create_args(parent: BasicActorRef) -> Self {
        ActorCreation {
            parent: Some(parent),
            r1: 0,
            spread: 0,
            receiving: false,
        }
    }
}

// implement the Actor trait
impl Actor for ActorCreation {
    type Msg = usize;

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Stop
    }

    fn recv(&mut self, ctx: &Context<usize>, msg: usize, sender: Sender) {
        // println!(
        //     "name: {}, msg: {}, self: {:#?}",
        //     ctx.myself.name(),
        //     msg,
        //     &self
        // );
        if self.receiving {
            if self.r1 == 0 {
                self.r1 = msg;
            } else {
                let r2 = msg;
                if let Some(parent) = &self.parent {
                    let result = 2 + self.r1 + r2;
                    assert_eq!(result, 1 << self.spread);
                    parent.try_tell(result, ctx.myself());
                } else {
                    let result = 1 + self.r1 + r2;
                    ctx.myself.parent().try_tell(result, ctx.myself());
                }
                ctx.stop(&ctx.myself);
            }
        } else {
            if msg == 1 {
                sender.as_ref().unwrap().try_tell(1usize, ctx.myself());
                ctx.stop(&ctx.myself);
            } else {
                self.spread = msg;
                self.receiving = true;

                let a1 = ctx
                    .actor_of::<ActorCreation>(&format!("{}-1", ctx.myself.name()))
                    .unwrap();
                let a2 = ctx
                    .actor_of::<ActorCreation>(&format!("{}-2", ctx.myself.name()))
                    .unwrap();

                a1.send_msg(msg - 1usize, ctx.myself());
                a2.send_msg(msg - 1usize, ctx.myself());
            }
        }
    }
}

struct Options {
    start_value: usize,
}

fn get_options() -> Options {
    let mut start_value = None;
    {
        // this block limits scope of borrows by ap.refer() method
        let mut ap = ArgumentParser::new();
        ap.set_description("Run riker test.");
        ap.refer(&mut start_value)
            .add_option(&["-s", "--start-value"], StoreOption, "how many actors to start");
        ap.parse_args_or_exit();
    }

    Options {
        start_value: start_value.unwrap_or(10),
    }
}

// start the system and create an actor
fn main() {
    let sys = ActorSystem::new().unwrap();

    sys.user_root().has_children();

    let act = sys.actor_of::<ActorCreation>("act").unwrap();

    act.tell(10usize, None);

    while sys.user_root().has_children() {}
}
