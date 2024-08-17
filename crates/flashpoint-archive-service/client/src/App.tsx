import { Outlet } from "react-router-dom";
import { Header } from "./components/Header";

export function App() {
    return (
        <div className="w-full h-full">
            <Header />
            <div className="h-full">
                <Outlet />
            </div>
        </div>
    );
}