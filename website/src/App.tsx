import { Routes, Route } from "react-router-dom";
import Navbar from "./components/Navbar";
import Hero from "./components/Hero";
import Features from "./components/Features";
import HowItWorks from "./components/HowItWorks";
import TheSpec from "./components/TheSpec";
import GettingStarted from "./components/GettingStarted";
import Footer from "./components/Footer";

function LandingPage() {
  return (
    <>
      <Hero />
      <Features />
      <HowItWorks />
      <TheSpec />
      <GettingStarted />
    </>
  );
}

export default function App() {
  return (
    <div className="min-h-screen bg-surface overflow-x-hidden">
      <Navbar />
      <Routes>
        <Route path="/" element={<LandingPage />} />
      </Routes>
      <Footer />
    </div>
  );
}
