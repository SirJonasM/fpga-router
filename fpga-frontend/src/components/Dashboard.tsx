import React, { useState, useEffect } from "react";
import { CaseSelector } from "./CaseSelector";
import { LineChart } from "./LineChart";

export const Dashboard: React.FC = () => {
	const [cases, setCases] = useState<[number, number][]>([]);
	const [selected, setSelected] = useState<[number, number] | null>(null);
	const [rows, setRows] = useState<any[]>([]);

	const fetchCases = async () => {
		const res = await fetch("http://localhost:3000/cases");
		const data = await res.json();
		setCases(data);
		if (!selected && data.length > 0) {
			setSelected(data[0]);
		}
	};

	const fetchData = async (selectedCase: [number, number]) => {
		const res = await fetch(`http://localhost:3000/case/${selectedCase[0]}/${selectedCase[1]}`);
		const data = await res.json();
		setRows(data);
	};

	// Load cases on mount
	useEffect(() => {
		fetchCases();
	}, []);

	// Load data whenever selected changes
	useEffect(() => {
		if (selected) {
			fetchData(selected);
		}
	}, [selected]);

	const handleReload = () => {
		fetchCases();
		if (selected) fetchData(selected);
	};

	return (
		<div className="w-screen h-screen p-6 bg-gray-900 text-gray-200 font-sans flex flex-col">
			<h1 className="text-3xl font-bold text-center mb-6 text-gray-100">
				FPGA Routing Dashboard
			</h1>

			{/* Toolbar */}
			<div className="flex items-center justify-between mb-6">
				<CaseSelector cases={cases} selected={selected} onSelect={setSelected} />
				<button
					onClick={handleReload}
					className="bg-teal-600 hover:bg-teal-500 text-white py-1 px-4 rounded"
				>
					Reload
				</button>
			</div>

			{/* Charts */}
			<div className="grid grid-rows-2 grid-cols-2 gap-6 flex-1">
				{[
					{ label: "Conflicts", yKey: "conflicts" },
					{ label: "Longest Path", yKey: "longest_path_cost" },
					{ label: "Duration (µs)", yKey: "duration" },
					{ label: "Wire Reuse", yKey: "wire_reuse" },
				].map((chart) => (
					<div
						key={chart.label}
						className="bg-gray-800 p-4 rounded-lg shadow flex flex-col justify-center items-center h-full"
					>
						<h2 className="text-lg font-semibold mb-2 text-gray-100 text-center">
							{chart.label}
						</h2>
						<div className="w-full flex-1 flex justify-center">
							<LineChart data={rows} label={chart.label} yKey={chart.yKey} />
						</div>
					</div>
				))}
			</div>
		</div>
	)

};

