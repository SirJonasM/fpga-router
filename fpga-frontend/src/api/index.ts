import axios from "axios";

const API_BASE = "http://localhost:3133";

export type SolverType = "SimpleSolver" | "SteinerSolver" | "SimpleSteinerSolver";

export type TestState =
	| { type: "Scheduled" }
	| { type: "Running" }
	| { type: "Successfull"; value: number }
	| { type: "Failed"; value: number };

export type Test = {
	id: number;
	percentage: number;
	dst: number;
	hist_factor: number;
	solver: string;
	state: TestState;
};

export type RowData = {
	conflicts: number;
	longest_path_cost: number;
	duration: number;
	wire_reuse: number;
	solver: string;
	// Add other fields if needed
}

export type JsonNode = {
	id: number;
	label: string;
	usage: number;
	typ: "LutInput" | "LutOutput" | "Default";
	signals: number[];
};

export type JsonEdge = {
	from: number;
	to: number;
	weight: number;
	signals: number[];
};

export type JsonGraph = {
	nodes: JsonNode[];
	edges: JsonEdge[];
};

export async function getAllTests(): Promise<Test[]> {
	const res = await axios.get(`${API_BASE}/tests`);
	return res.data;
}
export async function getTestData(id: number): Promise<RowData[]> {
	const res = await axios.get(`${API_BASE}/data/${id}`);
	return res.data;
}
export async function getTest(id: number): Promise<Test> {
	const res = await axios.get(`${API_BASE}/test/${id}`);
	return res.data;
}
export async function createTest(percentage: number, dst: number, hist_factor: number, solverType: SolverType) {
	const res = await axios.post(`${API_BASE}/test`, {
		percentage,
		dst,
		solver: solverType,
		hist_factor,
	});
	return res.data; // returns the test id
}

export async function scheduleTest(id: number) {
	const res = await axios.get(`${API_BASE}/schedule/${id}`);
	return res.data; // returns the test id
}
export async function getGraphResult(id: number): Promise<JsonGraph> {
	const res = await fetch(`${API_BASE}/result/${id}`);
	if (!res.ok) throw new Error(`Failed to fetch graph result: ${res.status}`);
	return res.json();
}

export async function deleteTest(id: number): Promise<void> {
	await axios.delete(`${API_BASE}/test/${id}`);
}
