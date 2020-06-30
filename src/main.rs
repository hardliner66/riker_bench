use rayon::prelude::*;

use riker::actors::*;

#[derive(Debug, Default)]
struct ActorCreation;

// implement the Actor trait
impl Actor for ActorCreation {
    type Msg = i32;

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Stop
    }

    fn recv(&mut self, ctx: &Context<i32>, msg: i32, _sender: Sender) {
        if msg > 0 {
            let a1 = ctx
                .actor_of::<ActorCreation>(&format!("{}-", ctx.myself.name()))
                .unwrap();
            a1.send_msg(msg - 1, ctx.myself());
        }

        ctx.stop(&ctx.myself);
    }
}

// start the system and create an actor
fn main() {
    let collect_after_creation = false;

    let sys = ActorSystem::new().unwrap();

    let base_count = 10000;

    println!("creating actors");

    let actors = (0..base_count).into_par_iter().map(|i| {
        sys.actor_of::<ActorCreation>(&format!("act-{}", i))
            .unwrap()
    });

    println!("after creation");

    if collect_after_creation {
        let actors: Vec<_> = actors.collect();

        actors.par_iter().for_each(|act| act.tell(500, None));
    } else {
        actors.for_each(|act| act.tell(50, None));
    }

    println!("after tell");

    while sys.user_root().has_children() {
        // in order to lower cpu usage, sleep here
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
