import { BrowserRouter, Routes, Route } from "react-router-dom";
import { GridPage } from "./pages/GridPage";
import { CasePage } from "./pages/CasePage";
import { ResultPage } from "./pages/ResultPage";

function App() {
	return (
		<BrowserRouter>
			<Routes>
				<Route path="/" element={<GridPage />} />
				<Route path="/case/:id" element={<CasePage />} />
				<Route path="/result/:id" element={<ResultPage />} />
			</Routes>
		  </BrowserRouter>
	);
}

export default App;
