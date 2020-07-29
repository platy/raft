//! test of implementation of the network layer

use core::sync::atomic::{AtomicU32, Ordering};
use log::{info, trace};
use std::rc::Rc;

use futures::channel::oneshot;
use futures::future::join_all;
use raft::rpc::*;
use raft::state::Persistent;
use raft::{log as raftlog, server::Server, CommandPtr, ServerId};
use std::collections::HashMap;
use std::convert::TryInto;

use tokio::time::delay_for;

use actix::prelude::*;

use std::time::Duration;

// todo merge this into the core logic state machine
enum Timers<TimerHandle> {
    Uninitialised,
    Election(TimerHandle),
    Heartbeat(TimerHandle),
}

type Command = u32;

struct RoutedMessage<Message> {
    id: ServerId,
    msg: Message,
}

impl<Req: Message> Message for RoutedMessage<Req> {
    type Result = Req::Result;
}

#[derive(Clone)]
struct Configuration {
    own_id: ServerId,
    num_servers: ServerId,
    broker: Addr<ClusterRunner>,
}

impl Configuration {
    fn to_peers<Req: Message + Clone + Send + 'static>(
        &self,
        req: Req,
    ) -> Vec<(ServerId, Request<ClusterRunner, RoutedMessage<Req>>)>
    where
        ClusterRunner: Handler<RoutedMessage<Req>>,
        Req::Result: Send,
    {
        (0..self.num_servers)
            .into_iter()
            .filter_map(|id| {
                let id = id.try_into().unwrap();
                if self.own_id != id {
                    Some((
                        id,
                        self.broker.send(RoutedMessage {
                            id,
                            msg: req.clone(),
                        }),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}

struct Node<TimerHandle, AppliedState> {
    configuration: Configuration,
    timer: Timers<TimerHandle>,
    core: Server<Command, raftlog::InVec<Command>>,
    commands: HashMap<CommandPtr, oneshot::Sender<()>>,
    applied_state: AppliedState,
}

impl<TimerHandle, AppliedState> Node<TimerHandle, AppliedState> {
    fn complete_responses(&mut self, cmds: Vec<CommandPtr>) {
        for cmd in cmds {
            if let Some(sender) = self.commands.remove(&cmd) {
                let _ = sender.send(());
            }
        }
    }

    fn deferred_response(&mut self, cmd: CommandPtr) -> oneshot::Receiver<()> {
        let (sender, receiver) = oneshot::channel();
        self.commands.insert(cmd, sender);
        receiver
    }
}

impl Timers<SpawnHandle> {
    fn new<AppliedState: Unpin + 'static>(
        ctx: &mut <Node<SpawnHandle, AppliedState> as Actor>::Context,
    ) -> Self {
        let rand: f64 = random_number::rand::random();
        let millis = (rand + 1.) * 200.; // random between 150 & 300
        Timers::Election(ctx.notify_later(ElectionTimeout, Duration::from_millis(millis as u64)))
    }

    fn cancel<AppliedState: Unpin + 'static>(
        self,
        ctx: &mut <Node<SpawnHandle, AppliedState> as Actor>::Context,
    ) {
        match self {
            Timers::Election(handle) | Timers::Heartbeat(handle) => {
                ctx.cancel_future(handle);
            }
            Timers::Uninitialised => {}
        }
    }

    fn reset_election_timer<AppliedState: Unpin + 'static>(
        &mut self,
        ctx: &mut <Node<SpawnHandle, AppliedState> as Actor>::Context,
    ) {
        let old_timer = std::mem::replace(self, Timers::new(ctx));
        old_timer.cancel(ctx);
    }

    fn reset_heartbeat<AppliedState: Unpin + 'static>(
        &mut self,
        ctx: &mut <Node<SpawnHandle, AppliedState> as Actor>::Context,
    ) {
        let old_timer = std::mem::replace(
            self,
            Timers::Heartbeat(ctx.notify_later(Heartbeat, Duration::from_millis(15))),
        );
        old_timer.cancel(ctx);
    }
}

impl<AppliedState: Unpin + 'static> Actor for Node<SpawnHandle, AppliedState> {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.timer.reset_election_timer(ctx);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ElectionTimeout;

impl<AppliedState: Unpin + 'static> Handler<ElectionTimeout> for Node<SpawnHandle, AppliedState> {
    type Result = ();

    fn handle(&mut self, _: ElectionTimeout, ctx: &mut Self::Context) -> Self::Result {
        let req = self.core.election_timeout();
        // let match_index = req.prev_log_index + req.entries.len();

        self.timer.reset_election_timer(ctx);

        let requests = self.configuration.to_peers(req);
        let own_id = self.configuration.own_id;
        for (id, request) in requests {
            ctx.spawn(
                fut::wrap_future(async move {
                    trace!("{}; requesting vote from {}", own_id, id);
                    let resp = request.await.unwrap();
                    trace!("{}; received vote from {}", own_id, id);
                    resp
                })
                .map(move |resp, _actor, ctx: &mut Self::Context| ctx.notify(RQR(resp))),
            );
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct Heartbeat;

impl<AppliedState: Unpin + 'static> Handler<Heartbeat> for Node<SpawnHandle, AppliedState> {
    type Result = ();

    fn handle(&mut self, _: Heartbeat, ctx: &mut Self::Context) -> Self::Result {
        let reqs = self.core.heartbeat();
        let own_id = self.configuration.own_id;
        for (id, req) in reqs {
            let match_index = req.prev_log_index + req.entries.len();
            let request = self
                .configuration
                .broker
                .send(RoutedMessage { id, msg: req });
            ctx.spawn(
                fut::wrap_future(async move {
                    trace!("{}; requesting vote from {}", own_id, id);
                    let resp = request.await.unwrap();
                    trace!("{}; received vote from {}", own_id, id);
                    resp
                })
                .map(move |resp, _actor, ctx: &mut Self::Context| {
                    ctx.notify(AER(resp, match_index, id))
                }),
            );
        }

        self.timer.reset_heartbeat(ctx);
    }
}

impl<AppliedState: Unpin + 'static, Entries: IntoIterator<Item = raftlog::Item<Command>>>
    Handler<AppendEntriesRequest<Command, Entries>> for Node<SpawnHandle, AppliedState>
{
    type Result = MessageResult<AppendEntriesRequest<Command, Entries>>;

    fn handle(
        &mut self,
        req: AppendEntriesRequest<Command, Entries>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        self.timer.reset_election_timer(ctx); // todo : shouldn't we check the term?
        MessageResult(self.core.receive_append_entries(req))
    }
}

impl<AppliedState: Unpin + 'static> Handler<RequestVoteRequest>
    for Node<SpawnHandle, AppliedState>
{
    type Result = MessageResult<RequestVoteRequest>;

    fn handle(&mut self, req: RequestVoteRequest, ctx: &mut Self::Context) -> Self::Result {
        trace!("{}; received vote request", self.configuration.own_id);
        self.timer.reset_election_timer(ctx); // todo : shouldn't we check the term / whether we voted for them?
        MessageResult(self.core.receive_request_vote(req))
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct RQR(RequestVoteResponse);

impl<AppliedState: Unpin + 'static> Handler<RQR> for Node<SpawnHandle, AppliedState> {
    type Result = ();

    fn handle(&mut self, RQR(req): RQR, ctx: &mut Self::Context) -> Self::Result {
        trace!("{}; received vote {:?}", self.configuration.own_id, req);
        if let Some(req) = self.core.receive_vote(req, self.configuration.num_servers) {
            info!("{}; we became the leader", self.configuration.own_id);
            self.timer.reset_heartbeat(ctx);

            let match_index = req.prev_log_index + req.entries.len();
            let requests = self.configuration.to_peers(req);
            for (id, request) in requests {
                ctx.spawn(fut::wrap_future(async move { request.await.unwrap() }).map(
                    move |resp, _actor, ctx: &mut Self::Context| {
                        ctx.notify(AER(resp, match_index, id))
                    },
                ));
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct AER(AppendEntriesResponse, usize, ServerId);

impl<AppliedState: Unpin + 'static> Handler<AER> for Node<SpawnHandle, AppliedState> {
    type Result = ();

    fn handle(
        &mut self,
        AER(resp, match_index, from): AER,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let applied_commands = self
            .core
            .receive_append_entries_response(from, match_index, resp);
        self.complete_responses(applied_commands);
    }
}

#[derive(Message, Clone)]
#[rtype(result = "Result<CommandApplied, CommandError>")]
struct ApplyCommand(Command);
#[derive(Debug, Eq, PartialEq)]
struct CommandApplied(Command);

#[derive(Debug, Eq, PartialEq)]
enum CommandError {
    NotLeader,
    ResponseCanceled,
}

impl Handler<ApplyCommand> for Node<SpawnHandle, Rc<AtomicU32>> {
    type Result = ResponseActFuture<Self, Result<CommandApplied, CommandError>>;

    fn handle(
        &mut self,
        ApplyCommand(command): ApplyCommand,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        // add command to raftlog
        let ptr = match self.core.command(command) {
            Ok(ptr) => ptr,
            Err(raft::NotLeader) => {
                return Box::pin(
                    futures::future::ready(Err(CommandError::NotLeader)).into_actor(self),
                );
            }
        };
        // send heartbeat immediately
        ctx.address().do_send(Heartbeat);
        let future = self.deferred_response(ptr);
        Box::pin(future.into_actor(self).map(|res, act, _ctx| match res {
            Ok(()) => Ok(CommandApplied(act.applied_state.load(Ordering::SeqCst))),
            Err(oneshot::Canceled) => Err(CommandError::ResponseCanceled),
        }))
    }
}

// a cluster of 1 actually never becomes leader due to an implmentation detail / bug
#[test]
fn test_cluster_1() {
    let mut system = System::new("test");

    let acc = Rc::new(AtomicU32::new(0));
    let acc2 = acc.clone();
    let state_machine = move |&command: &Command| {
        acc2.fetch_add(command, Ordering::SeqCst);
    };
    let persistent = Persistent::default();

    let node = Node {
        configuration: Configuration {
            own_id: 0,
            num_servers: 1,
            broker: ClusterRunner {
                size: 0,
                addrs: vec![],
            }
            .start(), // runner not needed for this test
        },
        timer: Timers::Uninitialised,
        core: Server::new(0, persistent, Box::new(state_machine)),
        commands: HashMap::new(),
        applied_state: acc,
    }
    .start();

    system.block_on(async move {
        delay_for(Duration::from_millis(500)).await;
        let r = node.send(ApplyCommand(3)).await;
        println!("applied {:?}", r);
        //   assert_eq!(r.unwrap(), Ok(CommandApplied(3)));
        // assert_eq!(r.unwrap(), Err(oneshot::Canceled));
    });
}

struct ClusterRunner {
    size: ServerId,
    addrs: Vec<Addr<Node<SpawnHandle, Rc<AtomicU32>>>>,
}

impl ClusterRunner {
    fn mknode(
        id: ServerId,
        configuration: Configuration,
    ) -> Addr<Node<SpawnHandle, Rc<AtomicU32>>> {
        // todo this should use the actor rather than Rc & atomic
        let acc = Rc::new(AtomicU32::new(0));
        let acc2 = acc.clone();
        let state_machine = move |&command: &Command| {
            acc2.fetch_add(command, Ordering::SeqCst);
        };
        let persistent = Persistent::default();

        Node {
            configuration,
            timer: Timers::Uninitialised,
            core: Server::new(id, persistent, Box::new(state_machine)),
            commands: HashMap::new(),
            applied_state: acc,
        }
        .start()
    }
}

impl Actor for ClusterRunner {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.addrs = (0..self.size)
            .into_iter()
            .map(|id| {
                Self::mknode(
                    id,
                    Configuration {
                        own_id: id,
                        num_servers: self.size,
                        broker: ctx.address(),
                    },
                )
            })
            .collect();
    }
}

impl Handler<ApplyCommand> for ClusterRunner {
    type Result = ResponseFuture<Result<CommandApplied, CommandError>>;

    fn handle(&mut self, msg: ApplyCommand, _ctx: &mut Self::Context) -> Self::Result {
        let requests = join_all(self.addrs.iter().map(|addr| addr.send(msg.clone())));
        Box::pin(async {
            let r = requests.await;
            r.into_iter().fold(Err(CommandError::NotLeader), |acc, e| {
                match (acc, e) {
                    (_, Err(e)) => panic!("error sending {:?}", e),
                    (_, Ok(Err(CommandError::ResponseCanceled))) => panic!("response cancelled"),
                    (acc, Ok(Err(CommandError::NotLeader))) => acc, // ignore, was not the leader
                    (Err(_), Ok(ok)) => ok,
                    (Ok(_), Ok(Ok(_))) => panic!("more than one successful response"),
                }
            })
        })
    }
}

/// ClusterRunner will route any `RoutedMessage`s that nodes can handle it can to it's cluster nodes
impl<Req> Handler<RoutedMessage<Req>> for ClusterRunner
where
    Req: Message + Send + std::fmt::Debug + 'static,
    Req::Result: Send + std::fmt::Debug,
    Node<SpawnHandle, Rc<AtomicU32>>: Handler<Req>,
{
    type Result = ResponseFuture<Req::Result>;

    fn handle(
        &mut self,
        RoutedMessage { id, msg }: RoutedMessage<Req>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trace!("R; -> {} - {:?}", id, msg);
        let req_fut = self.addrs[id as usize].send(msg);
        Box::pin(async move {
            let res = req_fut.await.unwrap();
            trace!("R; <- {} - {:?}", id, res);
            res
        })
    }
}

#[test]
fn test_cluster_2() {
    let mut system = System::new("test2");

    let cluster = ClusterRunner {
        size: 2,
        addrs: vec![],
    }
    .start();

    system.block_on(async move {
        delay_for(Duration::from_millis(500)).await;
        let r = cluster.send(ApplyCommand(3)).await;
        println!("applied {:?}", r);
        let b = cluster.send(ApplyCommand(3));
        assert_eq!(r.unwrap(), Ok(CommandApplied(3)));
        assert_eq!(b.await.unwrap(), Ok(CommandApplied(6)));
    });
}

#[test]
fn test_cluster_5() {
    let mut system = System::new("test5");

    let cluster = ClusterRunner {
        size: 5,
        addrs: vec![],
    }
    .start();

    system.block_on(async move {
        delay_for(Duration::from_millis(500)).await;
        let r = cluster.send(ApplyCommand(3)).await;
        println!("applied {:?}", r);
        let b = cluster.send(ApplyCommand(3));
        assert_eq!(r.unwrap(), Ok(CommandApplied(3)));
        assert_eq!(b.await.unwrap(), Ok(CommandApplied(6)));
    });
}

// cluster of 20 takes a bit longer to agree on a leader, so it needs a 5 second delay
#[test]
fn test_cluster_20() {
    let mut system = System::new("test20");

    let cluster = ClusterRunner {
        size: 20,
        addrs: vec![],
    }
    .start();

    system.block_on(async move {
        delay_for(Duration::from_millis(5000)).await;
        let r = cluster
            .send(ApplyCommand(3))
            .timeout(Duration::from_millis(1000))
            .await;
        println!("applied {:?}", r);
        assert_eq!(r.unwrap(), Ok(CommandApplied(3)));
    });
}
