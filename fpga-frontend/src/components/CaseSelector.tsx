import React from "react";
import "../App.css"
interface Props {
	cases: [number, number][]; // [[source_per, destinations], ...]
	selected: [number, number] | null;
	onSelect: (c: [number, number]) => void;
}

export const CaseSelector: React.FC<Props> = ({ cases, selected, onSelect }) => {
	const sourceOptions = Array.from(new Set(cases.map(([src, _]) => src)));
	const destOptions = Array.from(new Set(cases.map(([_, dst]) => dst)));

	const currentSource = selected ? selected[0] : sourceOptions[0];
	const currentDest = selected ? selected[1] : destOptions[0];

	const handleSourceChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
		const newSource = Number(e.target.value);
		onSelect([newSource, currentDest]);
	};

	const handleDestChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
		const newDest = Number(e.target.value);
		onSelect([currentSource, newDest]);
	};

	return (
        <div className="flex gap-8 items-center p-3 bg-gray-800 text-gray-200 rounded-md shadow-sm">

            {/* Source */}
            <div className="flex flex-col gap-2">
                <label className="text-sm font-medium text-gray-300">
                    Source %
                </label>
                <select
                    value={currentSource}
                    onChange={handleSourceChange}
                    className="px-4 py-2 border border-gray-600 rounded-md 
                               bg-gray-700 text-gray-100 shadow-sm 
                               focus:outline-none focus:ring-2 focus:ring-teal-400 
                               focus:border-teal-400"
                >
                    {sourceOptions.map((src) => (
                        <option key={src} value={src} className="bg-gray-700 text-gray-100">
                            {src}
                        </option>
                    ))}
                </select>
            </div>

            {/* Destination */}
            <div className="flex flex-col gap-2">
                <label className="text-sm font-medium text-gray-300">
                    Destinations
                </label>
                <select
                    value={currentDest}
                    onChange={handleDestChange}
                    className="px-4 py-2 border border-gray-600 rounded-md 
                               bg-gray-700 text-gray-100 shadow-sm 
                               focus:outline-none focus:ring-2 focus:ring-teal-400 
                               focus:border-teal-400"
                >
                    {destOptions.map((dst) => (
                        <option key={dst} value={dst} className="bg-gray-700 text-gray-100">
                            {dst}
                        </option>
                    ))}
                </select>
            </div>

        </div>
    );
};

