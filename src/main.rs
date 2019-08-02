#[macro_use]
extern crate log;

use std::sync::{Arc, Mutex, RwLock};

use argparse::{ArgumentParser, StoreOption};
use riker::actors::*;

struct Collector {
    cv: Arc<Mutex<std::sync::mpsc::Sender<()>>>,
    amount: usize,
    count: usize,
}

impl Collector {
    fn actor(args: (Arc<Mutex<std::sync::mpsc::Sender<()>>>, usize)) -> Self {
        let (cv, amount) = args;
        info!("Collector started, waiting for {} messages", amount);
        Collector {
            cv,
            amount,
            count: 0,
        }
    }

    fn props(cv: Arc<Mutex<std::sync::mpsc::Sender<()>>>, amount: usize) -> BoxActorProd<Collector> {
        Props::new_args(Collector::actor, (cv, amount))
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

// implement the Actor trait
impl Actor for MyActor {
    type Msg = String;

    fn recv(&mut self,
            ctx: &Context<String>,
            msg: String,
            _sender: Sender) {
        debug!("[{}] :: Received: {}", ctx.myself.name(), msg);
        self.collector.tell((), None);
    }
}

impl MyActor {
    fn actor(args: (usize, ActorRef<()>)) -> Self {
        let (index, collector) = args;
        info!("Actor({}) Started", index);
        MyActor {
            collector
        }
    }

    fn props(args: (usize, ActorRef<()>)) -> BoxActorProd<MyActor> {
        Props::new_args(MyActor::actor, args)
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
    let collector = sys.actor_of(Collector::props(Arc::new(Mutex::new(tx.clone())), amount), "collector").unwrap();

    info!("Starting {} Actors", actor_count);
    let actors = (0..actor_count).map(|i| sys.actor_of(MyActor::props((i, collector.clone())), &format!("my-actor-{}", i)).unwrap()).collect::<Vec<_>>();

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
                actor.get(index).unwrap().tell(format!("Hello from Sender({})", c), None);
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
