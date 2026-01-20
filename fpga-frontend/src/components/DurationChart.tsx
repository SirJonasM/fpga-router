import React from "react";
import { Line } from "react-chartjs-2";
import { Chart as ChartJS, Title, Tooltip, Legend, CategoryScale, LinearScale, PointElement, LineElement } from "chart.js";

ChartJS.register(Title, Tooltip, Legend, CategoryScale, LinearScale, PointElement, LineElement);

interface Props {
    data: any[];
    label: string;
    yKey: string;
}

export const LineChart: React.FC<Props> = ({ data, label, yKey }) => {
    const chartData = {
        labels: data.map((d) => d.iteration),
        datasets: [{ label, data: data.map((d) => d[yKey]), borderColor: "blue", fill: false }]
    };

    return <Line data={chartData} />;
};

