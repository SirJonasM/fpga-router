import React, { useEffect, useRef, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import {
  scheduleTest,
  getTest,
  getTestData,
  type RowData,
  type Test,
} from "../api";
import { LineChart } from "../components/LineChart";

const STATUS_POLL_INTERVAL = 2000;   // 2s
const DATA_POLL_INTERVAL = 15_000;   // 1 min

export const CasePage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const idNum = id ? Number(id) : null;
  const navigate = useNavigate();

  const [rows, setRows] = useState<RowData[]>([]);
  const [test, setTest] = useState<Test | null>(null);

  const statusPollRef = useRef<number | null>(null);
  const dataPollRef = useRef<number | null>(null);

  if (idNum === null) {
    return <div className="text-red-500">Invalid test case</div>;
  }

  // -------- Fetch status --------
  const fetchTest = async () => {
    try {
      const t = await getTest(idNum);
      setTest(t);

      if (t.state.type === "Running") {
        startDataPolling();
      } else {
        stopDataPolling();

        if (t.state.type === "Successfull" || t.state.type === "Failed") {
          const data = await getTestData(idNum);
          setRows(data);
          stopStatusPolling();
        }
      }
    } catch (err) {
      console.error(err);
    }
  };

  // -------- Fetch chart data --------
  const fetchData = async () => {
    try {
      const data = await getTestData(idNum);
      setRows(data);
    } catch (err) {
      console.error(err);
    }
  };

  // -------- Poll control --------
  const startStatusPolling = () => {
    if (!statusPollRef.current) {
      statusPollRef.current = window.setInterval(
        fetchTest,
        STATUS_POLL_INTERVAL
      );
    }
  };

  const stopStatusPolling = () => {
    if (statusPollRef.current) {
      clearInterval(statusPollRef.current);
      statusPollRef.current = null;
    }
  };

  const startDataPolling = () => {
    if (!dataPollRef.current) {
      fetchData(); // immediate fetch
      dataPollRef.current = window.setInterval(
        fetchData,
        DATA_POLL_INTERVAL
      );
    }
  };

  const stopDataPolling = () => {
    if (dataPollRef.current) {
      clearInterval(dataPollRef.current);
      dataPollRef.current = null;
    }
  };

  // -------- Initial mount --------
  useEffect(() => {
    fetchTest();
    startStatusPolling();

    return () => {
      stopStatusPolling();
      stopDataPolling();
    };
  }, [idNum]);

  const handleScheduleAgain = async () => {
    await scheduleTest(idNum);
    setRows([]);
    fetchTest();
    startStatusPolling();
  };

  // -------- Status badge --------
  const statusBadge = () => {
    if (!test) return null;

    switch (test.state.type) {
      case "Scheduled":
        return <span className="text-yellow-400">● Scheduled</span>;
      case "Running":
        return <span className="text-blue-400 animate-pulse">● Running…</span>;
      case "Successfull":
        return (
          <span className="text-green-400">
            ● Success ({test.state.value} iter)
          </span>
        );
      case "Failed":
        return (
          <span className="text-red-400">
            ● Failed ({test.state.value} conflicts)
          </span>
        );
    }
  };

  return (
    <div className="p-6 bg-gray-900 text-gray-200 min-h-screen flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-2xl font-bold">
          Test Case: {test?.percentage}% → {test?.dst} destinations | history factor: {test?.hist_factor} | Solver: {test?.solver}
        </h1>
        <div className="text-sm font-medium">{statusBadge()}</div>
      </div>

      {/* Actions */}
      <div className="flex gap-4 mb-6 justify-between">
        <button
          onClick={() => navigate("/")}
          className="bg-teal-900 hover:bg-teal-500 px-4 py-2 rounded w-40"
        >
          Back
        </button>

        <button
          onClick={handleScheduleAgain}
          className="bg-teal-600 hover:bg-teal-500 px-4 py-2 rounded w-40"
        >
          Schedule again
        </button>

        <button
          disabled={test?.state.type !== "Successfull"}
          onClick={() => navigate(`/result/${idNum}`)}
          className="bg-teal-600 hover:bg-teal-500 disabled:bg-gray-600
                     px-4 py-2 rounded w-40"
        >
          Visualize
        </button>
      </div>

      {/* Charts */}
      <div className="grid grid-rows-2 grid-cols-2 gap-6 flex-1">
        {[
          { label: "Conflicts", yKey: "conflicts" },
          { label: "Longest Path", yKey: "longest_path_cost" },
          { label: "Total Wire Use", yKey: "total_wire_use" },
          { label: "Wire Reuse", yKey: "wire_reuse" },
        ].map(chart => (
          <div
            key={chart.label}
            className="bg-gray-800 p-4 rounded-lg shadow flex flex-col"
          >
            <div className="w-full flex-1 flex justify-center">
              <LineChart data={rows} label={chart.label} yKey={chart.yKey} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

