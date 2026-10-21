import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/Shell";
import Audit from "./pages/Audit";
import Docs from "./pages/Docs";
import Landing from "./pages/Landing";
import List from "./pages/List";
import Portfolio from "./pages/Portfolio";
import Rounds from "./pages/Rounds";
import Trade from "./pages/Trade";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Landing />} />
        <Route element={<AppShell />}>
          <Route path="/rounds" element={<Rounds />} />
          <Route path="/rounds/:id" element={<Rounds />} />
          <Route path="/trade" element={<Trade />} />
          <Route path="/list" element={<List />} />
          <Route path="/portfolio" element={<Portfolio />} />
          <Route path="/audit" element={<Audit />} />
        </Route>
        <Route path="/docs" element={<Docs />} />
        <Route path="/docs/:slug" element={<Docs />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
