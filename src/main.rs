#![allow(unused)]

#[macro_use]
#[cfg(feature = "logging")]
extern crate log;

#[cfg(not(feature = "logging"))]
macro_rules! info {
    ($e:expr) => {};

    ($e:expr, $($es:expr),+) => {{
        info! { $e }
        info! { $($es),+ }
    }};
}

#[cfg(not(feature = "logging"))]
macro_rules! debug {
    ($e:expr) => {};

    ($e:expr, $($es:expr),+) => {{
        debug! { $e }
        debug! { $($es),+ }
    }};
}

use std::sync::{Arc, Mutex, RwLock};

use argparse::{ArgumentParser, StoreOption};
use riker::actors::*;

struct Collector {
    cv: Arc<Mutex<std::sync::mpsc::Sender<()>>>,
    amount: usize,
    count: usize,
}

impl ActorFactoryArgs<(Arc<Mutex<std::sync::mpsc::Sender<()>>>, usize)> for Collector {
    fn create_args((cv, amount): (Arc<Mutex<std::sync::mpsc::Sender<()>>>, usize)) -> Self {
        if cfg!(feature = "logging") {
            info!("Collector started, waiting for {} messages", amount);
        }
        Collector {
            cv,
            amount,
            count: 0,
        }
    }
}

// implement the Actor trait
impl Actor for Collector {
    type Msg = ();

    fn recv(&mut self,
            _ctx: &Context<()>,
            _msg: (),
            _sender: Sender) {
        self.count += 1;
        if self.count >= self.amount {
            let cv = self.cv.lock().unwrap();
            cv.send(()).unwrap();
        }
    }
}

struct MyActor {
    collector: ActorRef<()>,
}

impl ActorFactoryArgs<(usize, ActorRef<()>)> for MyActor {
    fn create_args((index, collector): (usize, ActorRef<()>)) -> Self {
        info!("Actor({}) Started", index);
        MyActor {
            collector
        }
    }
}

// implement the Actor trait
impl Actor for MyActor {
    type Msg = usize;

    fn recv(&mut self,
            ctx: &Context<usize>,
            msg: usize,
            _sender: Sender) {
        debug!("[{}] :: Received: {}", ctx.myself.name(), msg);
        self.collector.tell((), None);
    }
}

struct Options {
    actor_count: usize,
    message_count: usize,
}

fn get_options() -> Options {
    let mut actor_count = None;
    let mut message_count = None;
    {
        // this block limits scope of borrows by ap.refer() method
        let mut ap = ArgumentParser::new();
        ap.set_description("Run riker test.");
        ap.refer(&mut actor_count)
            .add_option(&["-a", "--actor-count"], StoreOption, "how many actors to start");
        ap.refer(&mut message_count)
            .add_option(&["-m", "--message-count"], StoreOption, "how many messages to send");
        ap.parse_args_or_exit();
    }

    let actor_count = actor_count.unwrap_or(num_cpus::get_physical());
    Options {
        actor_count,
        message_count: message_count.unwrap_or(actor_count * 1000),
    }
}

// start the system and create an actor
fn main() {
    env_logger::init();

    let options = get_options();

    let sys = ActorSystem::new().unwrap();

    let sender_count = num_cpus::get_physical();

    let (tx, rx) = std::sync::mpsc::channel();
    let amount = options.message_count - (options.message_count % sender_count);
    let actor_count = options.actor_count;

    info!("Message count: {}", amount);

    info!("Starting Collector");
    let collector = sys.actor_of_args::<Collector, _>("collector", (Arc::new(Mutex::new(tx.clone())), amount)).unwrap();

    info!("Starting {} Actors", actor_count);
    let actors = (0..actor_count).map(|i| sys.actor_of_args::<MyActor, _>(&format!("my-actor-{}", i), (i, collector.clone())).unwrap()).collect::<Vec<_>>();

    let actors = Arc::new(RwLock::new(actors));

    let amount_per_thread = amount / sender_count;

    info!("Starting {} Sender Threads", sender_count);
    for c in 0..sender_count {
        let actors_clone = actors.clone();
        std::thread::spawn(move || {
            for i in c..amount_per_thread + c {
                let actor = actors_clone.read().unwrap();
                let index = i % actor_count;
                debug!("Sender({}) sending Message to Actor({})", c, index);
                actor.get(index).unwrap().tell( c, None);
            }
        });
    }

    info!("Waiting for all actors to finish");
    let _ = rx.recv();

    info!("Shutting down");
    let mut r = sys.shutdown();
    loop {
        if let Ok(Some(())) = r.try_recv() {
            break;
        }
    }
}
