import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
	getAllTests,
	createTest,
	scheduleTest,
	deleteTest,
	type Test,
	type SolverType,
} from "../api";

type ContextMenuState = {
	x: number;
	y: number;
	test: Test;
} | null;


export const GridPage: React.FC = () => {
	const [tests, setTests] = useState<Test[]>([]);
	const [showDialog, setShowDialog] = useState(false);
	const [percentage, setPercentage] = useState(50);
	const [histFactor, setHistFactor] = useState(0.001);
	const [solverType, setSolverType] = useState<SolverType>(
		"SimpleSolver"
	);
	const [dst, setDst] = useState(1);
	const [contextMenu, setContextMenu] = useState<ContextMenuState>(null);

	const navigate = useNavigate();

	useEffect(() => {
		getAllTests().then(setTests);
	}, []);

	// ---------- Scheduling ----------
	const handleScheduleNew = async () => {
		const id = await createTest(percentage, dst, histFactor, solverType);
		await scheduleTest(id);

		setTests(prev => [
			...prev,
			{
				id,
				percentage,
				dst,
				hist_factor: histFactor,
				solver: solverType,
				state: { type: "Scheduled" },
			},
		]);

		setShowDialog(false);
	};

	const handleReschedule = async (test: Test) => {
		await scheduleTest(test.id);

		setTests(prev =>
			prev.map(t =>
				t.id === test.id
					? { ...t, state: { type: "Scheduled" } }
					: t
			)
		);
	};

	// ---------- Delete ----------
	const handleDelete = async (id: number) => {
		await deleteTest(id);
		setTests(prev => prev.filter(t => t.id !== id));
		setContextMenu(null);
	};

	// ---------- UI helpers ----------
	const stateStyle = (test: Test) => {
		switch (test.state.type) {
			case "Scheduled":
				return "bg-yellow-600";
			case "Running":
				return "bg-blue-700";
			case "Successfull":
				return "bg-green-700";
			case "Failed":
				return "bg-red-700";
		}
	};

	const stateLabel = (test: Test) => {
		switch (test.state.type) {
			case "Scheduled":
				return "Scheduled";
			case "Running":
				return "Running…";
			case "Successfull":
				return `${test.state.value} iter`;
			case "Failed":
				return `${test.state.value} conflicts`;
		}
	};

	return (
		<div className="w-screen h-screen p-6 bg-gray-900 text-gray-100">
			<h1 className="text-3xl font-bold mb-6 text-center">
				FPGA Routing Tests
			</h1>

			{/* Grid */}
			<div className="grid grid-cols-8 gap-4">
				{tests.map(test => (
					<div
						key={test.id}
						onClick={() => navigate(`/case/${test.id}`)}
						onContextMenu={e => {
							e.preventDefault();
							setContextMenu({
								x: e.clientX,
								y: e.clientY,
								test,
							});
						}}
						className={`cursor-pointer rounded-lg p-4 text-center shadow
              hover:scale-105 transition ${stateStyle(test)}`}
					>
						<div className="text-sm">
							{test.percentage}% · dst {test.dst}
						</div>
						<div className="font-semibold">
							{stateLabel(test)}
						</div>
						<div className="text-sm">
							{test.hist_factor}
						</div>
					</div>
				))}

				{/* Add new */}
				<div
					onClick={() => setShowDialog(true)}
					className="cursor-pointer rounded-lg p-4 flex items-center justify-center
                     border-2 border-dashed border-gray-500 hover:border-teal-400
                     text-4xl text-gray-400 hover:text-teal-400"
				>
					+
				</div>
			</div>

			{/* Context menu */}
			{contextMenu && (
				<>
					<div
						className="fixed inset-0 z-40"
						onClick={() => setContextMenu(null)}
					/>

					<div
						className="fixed z-50 bg-gray-800 border border-gray-700
                       rounded-md shadow-lg text-sm w-40"
						style={{ top: contextMenu.y, left: contextMenu.x }}
					>
						<MenuItem
							label="Restart"
							onClick={() => {
								handleReschedule(contextMenu.test);
								setContextMenu(null);
							}}
						/>
						<MenuItem
							label="Plots"
							onClick={() => {
								navigate(`/case/${contextMenu.test.id}`);
								setContextMenu(null);
							}}
						/>
						<MenuItem
							label="Visualize"
							onClick={() => {
								navigate(`/result/${contextMenu.test.id}`);
								setContextMenu(null);
							}}
						/>
						<MenuItem
							label="Delete"
							danger
							onClick={() => handleDelete(contextMenu.test.id)}
						/>
					</div>
				</>
			)}

			{/* Schedule dialog */}
			{showDialog && (
				<div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
					<div className="bg-gray-800 p-6 rounded-lg w-80">
						<h2 className="text-xl font-bold mb-4">New Test</h2>

						<label className="block mb-2 text-sm">Percentage</label>
						<input
							type="number"
							value={percentage}
							onChange={e => setPercentage(+e.target.value)}
							className="w-full mb-4 px-3 py-2 rounded bg-gray-700"
						/>

						<label className="block mb-2 text-sm">Destinations</label>
						<input
							type="number"
							value={dst}
							onChange={e => setDst(+e.target.value)}
							className="w-full mb-4 px-3 py-2 rounded bg-gray-700"
						/>

						<label className="block mb-2 text-sm">Historical Factor</label>
						<input
							type="number"
							value={histFactor}
							onChange={e => setHistFactor(+e.target.value)}
							className="w-full mb-4 px-3 py-2 rounded bg-gray-700"
						/>
						<div className="flex justify-evenly items-center gap-6">
							{/* Simple Solver */}
							<label className="flex items-center gap-2 cursor-pointer">
								<input
									type="radio"
									name="solver"
									value="SimpleSolver"
									checked={solverType === "SimpleSolver"}
									onChange={() => setSolverType("SimpleSolver")}
									className="accent-teal-500"
								/>
								<span>Simple Solver</span>
							</label>

							{/* Steiner Solver */}
							<label className="flex items-center gap-2 cursor-pointer">
								<input
									type="radio"
									name="solver"
									value="SteinerSolver"
									checked={solverType === "SteinerSolver"}
									onChange={() => setSolverType("SteinerSolver")}
									className="accent-teal-500"
								/>
								<span>Steiner Solver</span>
							</label>
							<label className="flex items-center gap-2 cursor-pointer">
								<input
									type="radio"
									name="solver"
									value="SimpleSteinerSolver"
									checked={solverType === "SimpleSteinerSolver"}
									onChange={() => setSolverType("SimpleSteinerSolver")}
									className="accent-teal-500"
								/>
								<span>Simple Steiner Solver</span>
							</label>
						</div>
						<div className="flex justify-end gap-3">
							<button
								onClick={() => setShowDialog(false)}
								className="px-3 py-1 bg-gray-600 rounded"
							>
								Cancel
							</button>
							<button
								onClick={handleScheduleNew}
								className="px-3 py-1 bg-teal-600 rounded"
							>
								Create & Schedule
							</button>
						</div>
					</div>
				</div>
			)}
		</div>
	);
};

const MenuItem: React.FC<{
	label: string;
	onClick: () => void;
	danger?: boolean;
}> = ({ label, onClick, danger }) => (
	<div
		onClick={onClick}
		className={`px-4 py-2 cursor-pointer
      ${danger
				? "text-red-400 hover:bg-red-600/20"
				: "hover:bg-gray-700"}
    `}
	>
		{label}
	</div>
);

