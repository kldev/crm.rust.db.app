import { useEffect, useState } from "react";

/**
 * The current time, ticking every `intervalMs` (30 s by default). Deadline states are a function of time, so pages
 * that show them re-render with this clock and a warning turns into an alert by itself (ADR-0007).
 */
export function useNow(intervalMs = 30_000): number {
	const [now, setNow] = useState(() => Date.now());
	useEffect(() => {
		const id = window.setInterval(() => setNow(Date.now()), intervalMs);
		return () => window.clearInterval(id);
	}, [intervalMs]);
	return now;
}
