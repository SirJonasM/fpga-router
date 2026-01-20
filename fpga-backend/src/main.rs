use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tower_http::cors::{Any, CorsLayer};

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use router::{IterationResult, SimpleSolver, SimpleSteinerSolver, Solver, SteinerSolver};
use router::{FabricGraph, Routing, TestCase, export_steiner_to_json, validate_routing};

use router::{Logging, route};

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(tag = "type", content = "value")]
pub enum TestState {
    Scheduled,
    Successfull(usize),
    Failed(usize),
    Running,
    #[default]
    Undefined,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Test {
    pub id: u64,
    pub percentage: usize,
    pub dst: usize,
    pub hist_factor: f32,
    pub solver: SolverType,
    pub state: TestState,
}

pub struct AppState {
    pub next_id: AtomicU64,
    pub data: RwLock<HashMap<u64, Vec<IterationResult>>>,
    pub results: RwLock<HashMap<u64, Vec<Routing>>>,
    pub tests: RwLock<HashMap<u64, Test>>,
    pub schedule_queue: RwLock<VecDeque<u64>>,
    pub runner_semaphore: Semaphore,
}
impl Logging for AppState {
    fn log(&self, log_instance: &IterationResult) {
        self.insert(log_instance.clone());
    }
}
impl AppState {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(0),
            data: RwLock::new(HashMap::new()),
            results: RwLock::new(HashMap::new()),
            tests: RwLock::new(HashMap::new()),
            schedule_queue: RwLock::new(VecDeque::new()),
            runner_semaphore: Semaphore::new(5),
        }
    }

    pub fn insert(&self, row: IterationResult) {
        let key = row.test_case.id;
        let mut map = self.data.write().unwrap();
        map.entry(key).or_default().push(row.clone());
    }

    pub fn create_test(
        &self,
        percentage: usize,
        dst: usize,
        hist_factor: f32,
        solver: SolverType,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let test = Test {
            id,
            percentage,
            dst,
            hist_factor,
            solver,
            state: TestState::Scheduled,
        };
        self.tests.write().unwrap().insert(id, test);
        self.data.write().unwrap().insert(id, Vec::new());
        id
    }
    pub fn schedule_test(&self, id: u64) -> Result<u64, String> {
        if self.tests.read().unwrap().contains_key(&id) {
            self.schedule_queue.write().unwrap().push_back(id);
            return Ok(id);
        }
        Err("Test does not exist.".to_string())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
async fn get_tests(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let tests = match app_state.tests.read() {
        Ok(tests) => tests,
        Err(_) => {
            return Json(vec![Test::default()]);
        }
    };

    let response = tests.values().cloned().collect::<Vec<Test>>();

    Json(response)
}
async fn get_test(Path(id): Path<u64>, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let map = state.tests.read().unwrap();
    let rows = map.get(&id).cloned().unwrap_or_default();
    Json(rows)
}
async fn get_data(Path(id): Path<u64>, State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, impl IntoResponse>{
    let data = state.data.read().unwrap();
    let rows = match data.get(&id){
        Some(rows) => rows,
        None => {
        let err = ErrorResponse {
            message: format!("Results with id {} not found", id),
        };
        return Err((StatusCode::NOT_FOUND, Json(err)))
        },
    };
    let rows = if rows.len()> 1000 {
        rows[rows.len()-1000..].to_vec()
    }else {
        rows.clone()
    };

    Ok(Json(rows))
}
async fn get_result(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    if let Some(result) = state.results.read().unwrap().get(&id) {
        let graph = get_graph();
        let result = export_steiner_to_json(&graph, result);
        Ok(Json(result))
    } else {
        let err = ErrorResponse {
            message: format!("Result with id {} not found", id),
        };
        Err((StatusCode::NOT_FOUND, Json(err)))
    }
}

async fn delete_test(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let tests = state.tests.read().unwrap().clone();
    let test = match tests.get(&id) {
        Some(test) => test,
        None => {
            let err = ErrorResponse {
                message: format!("Result with id {} not found", id),
            };
            return Err((StatusCode::NOT_FOUND, Json(err)));
        }
    };

    let r = match test.state {
        TestState::Successfull(_) | TestState::Failed(_) => {
            state.data.write().unwrap().remove(&id);
            state.tests.write().unwrap().remove(&id)
        }
        TestState::Scheduled => {
            let mut schedule_queue = state.schedule_queue.write().unwrap();
            let index = schedule_queue
                .iter()
                .enumerate()
                .find_map(|(i, a)| if *a == id { Some(i) } else { None });
            if let Some(index) = index {
                schedule_queue.remove(index);
                state.data.write().unwrap().remove(&id);
                state.tests.write().unwrap().remove(&id)
            } else {
                None
            }
        }
        TestState::Running | TestState::Undefined => None,
    };
    match r {
        Some(_) => Ok(StatusCode::NO_CONTENT),
        None => {
            let err = ErrorResponse {
                message: "Test with id is currently not deletable.".to_string(),
            };
            Err((StatusCode::NOT_FOUND, Json(err)))
        }
    }
}
#[derive(Deserialize)]
pub struct CreateTestRequest {
    percentage: usize,
    dst: usize,
    hist_factor: f32,
    solver: SolverType,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub enum SolverType {
    #[default]
    SimpleSolver,
    SteinerSolver,
    SimpleSteinerSolver,
}

async fn schedule_test(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let tests = state.tests.read().unwrap();
    let test = match tests.get(&id) {
        Some(test) => test,
        None => {
            let err = ErrorResponse {
                message: format!("Test with id {} not found", id),
            };
            return Err((StatusCode::NOT_FOUND, Json(err)));
        }
    };

    let mut data = state.data.write().unwrap();
    if let Some(data) = data.get_mut(&test.id) {
        data.clear();
    } else {
        data.insert(id, Vec::new());
    }
    state.schedule_queue.write().unwrap().push_back(test.id);
    Ok(Json(id))
}

pub async fn create_test(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateTestRequest>,
) -> impl IntoResponse {
    let id = app_state.create_test(
        payload.percentage,
        payload.dst,
        payload.hist_factor,
        payload.solver,
    );
    Json(id)
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(AppState::new());
    {
        let app_state = app_state.clone();
        tokio::spawn(async move {
            runner(app_state).await;
        });
    }
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    let app = Router::new()
        .route("/tests", get(get_tests))
        .route("/test", post(create_test))
        .route("/test/{id}", get(get_test))
        .route("/test/{id}", delete(delete_test))
        .route("/data/{id}", get(get_data))
        .route("/result/{id}", get(get_result))
        .route("/schedule/{id}", get(schedule_test))
        .layer(cors)
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3133));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Server running at {}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn runner(app_state: Arc<AppState>) {
    loop {
        let test_id_opt = {
            let mut queue = app_state.schedule_queue.write().unwrap();
            queue.pop_front()
        };

        if let Some(test_id) = test_id_opt {
            let app_state = app_state.clone();

            tokio::spawn(async move {
                let _permit = app_state.runner_semaphore.acquire().await.unwrap();

                // 1️⃣ Mark as Running
                let test = {
                    let mut tests = app_state.tests.write().unwrap();
                    let test = tests.get_mut(&test_id).expect("Test not found");
                    test.state = TestState::Running;
                    test.clone()
                };

                // 2️⃣ Run the test
                let result = run_test(test.clone(), app_state.clone()).await;

                // 3️⃣ Mark as Finished
                {
                    let mut tests = app_state.tests.write().unwrap();
                    if let Some(test) = tests.get_mut(&test_id) {
                        test.state = match result {
                            Ok(iterations) => {
                                app_state
                                    .results
                                    .write()
                                    .unwrap()
                                    .entry(test.id)
                                    .and_modify(|a| *a = iterations.1.clone())
                                    .or_insert_with(|| iterations.1.clone());
                                TestState::Successfull(iterations.0.iteration)
                            }
                            Err(conflicts) => TestState::Failed(conflicts),
                        }
                    }
                }
            });
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
async fn run_test(test: Test, app_state: Arc<AppState>) -> Result<(IterationResult, Vec<Routing>), usize> {
    tokio::task::spawn_blocking(move || {
        let mut graph = get_graph();
        let mut route_plan = graph.route_plan(test.percentage as f32 / 100.0, test.dst);
        let solver = match test.solver {
            SolverType::SimpleSolver => Solver::Simple(SimpleSolver),
            SolverType::SteinerSolver => Solver::Steiner(SteinerSolver),
            SolverType::SimpleSteinerSolver => Solver::SimpleSteiner(SimpleSteinerSolver),
        };
        let test_case = TestCase {
            id: test.id,
            percentage: test.percentage,
            dst: test.dst,
            hist_factor: test.hist_factor,
            solver
        };

        let result = route(
            &*app_state,
            test_case,
            &mut graph,
            &mut route_plan,
        )
        .unwrap();
        validate_routing(&graph, &route_plan).unwrap();
        Ok((result, route_plan))
    })
    .await
    .unwrap()
}
fn get_graph() -> FabricGraph {
    FabricGraph::from_file("../pips.txt").unwrap()
}
