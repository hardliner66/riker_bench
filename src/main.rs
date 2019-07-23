use std::sync::{Arc, Mutex};

use riker::actors::*;

struct Collector {
    cv: Arc<Mutex<std::sync::mpsc::Sender<()>>>,
    amount: usize,
    count: usize,
}

impl Collector {
    fn actor(args: (Arc<Mutex<std::sync::mpsc::Sender<()>>>, usize)) -> Self {
        let (cv, amount) = args;
        Collector {
            cv,
            amount,
            count: 0,
        }
    }

    fn props(cv: Arc<Mutex<std::sync::mpsc::Sender<()>>>, amount: usize) -> BoxActorProd<Collector> {
        Props::new_args(Box::new(Collector::actor), (cv, amount))
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
        println!("[{}] :: Received: {}", ctx.myself.name(), msg);
        self.collector.tell((), None);
    }
}

impl MyActor {
    fn actor(collector: ActorRef<()>) -> Self {
        MyActor {
            collector
        }
    }

    fn props(collector: ActorRef<()>) -> BoxActorProd<MyActor> {
        Props::new_args(Box::new(MyActor::actor), collector)
    }
}

const ACTOR_COUNT: usize = 40;
const MESSAGE_COUNT: usize = 40000;

// start the system and create an actor
fn main() {
    let sys = ActorSystem::new().unwrap();

    let (tx, rx) = std::sync::mpsc::channel();
    let amount = MESSAGE_COUNT;

    let collector = sys.actor_of(Collector::props(Arc::new(Mutex::new(tx)), amount), "collector").unwrap();

    let actors = (0..ACTOR_COUNT).map(|i| sys.actor_of(MyActor::props(collector.clone()), &format!("my-actor-{}", i)).unwrap()).collect::<Vec<_>>();

    for i in 0..MESSAGE_COUNT {
        actors.get(i % ACTOR_COUNT).unwrap().tell("Hello my actor!".to_string(), None);
    }

    let _ = rx.recv();

    let mut r = sys.shutdown();
    loop {
        if let Ok(Some(())) = r.try_recv() {
            break;
        }
    }
}