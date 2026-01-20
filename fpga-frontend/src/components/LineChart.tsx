import React, { useMemo, useRef } from "react";
import { Line } from "react-chartjs-2";
import {
  Chart as ChartJS,
  Title,
  Tooltip,
  Legend,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
} from "chart.js";
import jsPDF from "jspdf";

ChartJS.register(
  Title,
  Tooltip,
  Legend,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement
);

interface Props {
  data: any[];
  label: string;
  yKey: string;
}

export const LineChart: React.FC<Props> = ({ data, label, yKey }) => {
  const chartRef = useRef<any>(null);

  const chartData = useMemo(() => ({
    labels: data.map((d) => d.iteration),
    datasets: [
      {
        label,
        data: data.map((d) => d[yKey]),
        borderColor: "turquoise",
        borderWidth: 0.4,
        pointRadius: 0,
      },
    ],
  }), [data, label, yKey]);

  // ---------- Export PNG ----------
  const exportPNG = () => {
    const chart = chartRef.current;
    if (!chart) return;

    const url = chart.toBase64Image("image/png", 1);
    const link = document.createElement("a");
    link.href = url;
    link.download = `${label}.png`;
    link.click();
  };

  // ---------- Export PDF ----------
  const exportPDF = () => {
    const chart = chartRef.current;
    if (!chart) return;

    const pdf = new jsPDF();
    const imgData = chart.toBase64Image("image/png", 1);
    pdf.addImage(imgData, "PNG", 10, 10, 180, 100);
    pdf.save(`${label}.pdf`);
  };

  return (
    <div className="w-full h-full flex flex-col">
      {/* Chart */}
      <div className="flex-1">
        <Line
          ref={chartRef}
          data={chartData}
          options={{
            responsive: true,
            maintainAspectRatio: false,
          }}
        />
      </div>

      {/* Export buttons */}
      <div className="flex justify-end gap-2 mt-2">
        <button
          onClick={exportPNG}
          className="text-xs px-2 py-1 bg-gray-700 rounded hover:bg-gray-600"
        >
          PNG
        </button>
        <button
          onClick={exportPDF}
          className="text-xs px-2 py-1 bg-gray-700 rounded hover:bg-gray-600"
        >
          PDF
        </button>
      </div>
    </div>
  );
};

