import { useEffect, useRef, useState } from 'react';
import { Stdb } from './Stdb';

// Configuration
const TEST_DURATION_MS = 4 * 60 * 1000; // 4 minutes total
const PING_INTERVAL_MS = 200;
const VIDEO_SIM_INTERVAL_MS = 67; 
const FRAME_SIZE_BYTES = 20 * 1024; 
const FRAME_SIZE_FLOATS = FRAME_SIZE_BYTES / 4; 


function App() {
  const [mode, setMode] = useState<'none' | 'vanilla' | 'proxy'>('none');
  const [status, setStatus] = useState<string>('Idle');
  const pingTimer = useRef<number | null>(null);
  const loadTimer = useRef<number | null>(null);
  //const vanilla_stdb = useRef(new Stdb("wss://maincloud.spacetimedb.com", 'benchtimedb'));
  const proxy_stdb = useRef(new Stdb("ws://localhost:3001", 'benchtimedb'));

  // const vanillaOffset = useRef<number>(0);
  // const proxyOffset = useRef<number>(0);

  useEffect(() => {
    // Sync vanilla server time
    // vanilla_stdb.current.addEventListener("onApplied", () => {
    //   const row = vanilla_stdb.current.conn.db.clockSync.identity.find(vanilla_stdb.current.identity);
    //   if (row) {
    //     const clientNow = Date.now() / 1000;
    //     vanillaOffset.current = row.clock - clientNow;
    //     console.log("Vanilla offset:", vanillaOffset.current.toFixed(3));
    //   }
    // });

    // Sync proxy server time
    // proxy_stdb.current.addEventListener("onApplied", () => {
    //   const row = proxy_stdb.current.conn.db.clockSync.identity.find(proxy_stdb.current.identity);
    //   if (row) {
    //     const clientNow = Date.now() / 1000;
    //     proxyOffset.current = row.clock - clientNow;
    //     console.log("Proxy offset:", proxyOffset.current.toFixed(3));
    //   }
    // });
  }, []);

  const startTest = (useProxy: boolean) => {
    //const stdb = useProxy ? proxy_stdb.current : vanilla_stdb.current;
    const stdb = proxy_stdb.current;
    //const offset = useProxy ? proxyOffset.current : vanillaOffset.current;
    const label = useProxy ? 'Proxy Mode' : 'Vanilla Mode';
    setMode(useProxy ? 'proxy' : 'vanilla');
    setStatus(`${label}: Phase 1 - Latency Test`);
    let underLoad = false;

    pingTimer.current = window.setInterval(() => {
      //const syncedNow = (Date.now() / 1000) + offset;
      const syncedNow = Date.now() / 1000;
      stdb.conn.reducers.addLog(syncedNow, underLoad);
    }, PING_INTERVAL_MS);

    window.setTimeout(() => {
      setStatus(`${label}: Phase 2 - Load Test`);
      underLoad = true;

      if (pingTimer.current) {
        window.clearInterval(pingTimer.current);
        pingTimer.current = null;
      }

      loadTimer.current = window.setInterval(() => {
        const mockData = Array.from({ length: FRAME_SIZE_FLOATS }, () => Math.floor(Math.random() * 256));
        stdb.conn.reducers.addData(mockData);


        //const syncedNow = (Date.now() / 1000) + offset;
        const syncedNow = Date.now() / 1000;
        stdb.conn.reducers.addLog(syncedNow, underLoad);
      }, VIDEO_SIM_INTERVAL_MS);
    }, TEST_DURATION_MS / 2);

    window.setTimeout(() => {
      if (pingTimer.current) window.clearInterval(pingTimer.current);
      if (loadTimer.current) window.clearInterval(loadTimer.current);
      setStatus(`${label}: Test Complete`);
      setMode('none');
    }, TEST_DURATION_MS);
  };

  const exportData = () => {
    const data = Array.from(proxy_stdb.current.conn.db.logs.iter());
    const csvContent = [
      ['Id', 'Sent', 'Received', 'Latency', 'Jitter', 'UnderLoad'],
      ...data.map(log => [
        log.id,
        log.sent.toFixed(6),
        log.received.toFixed(6),
        log.latency.toFixed(6),
        log.jitter.toFixed(6),
        log.underLoad
      ])
    ].map(e => e.join(",")).join("\n");

    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "benchmark_logs.csv";
    link.click();
  };

  return (
    <div style={{ padding: 20, fontFamily: 'sans-serif' }}>
      <h1>SpacetimeDB Benchmark</h1>
      <p>Status: <strong>{status}</strong></p>

      <button onClick={() => startTest(false)} disabled={mode !== 'none'}>
        Start Vanilla SpacetimeDB Test
      </button>

      <button onClick={() => startTest(true)} disabled={mode !== 'none'} style={{ marginLeft: 10 }}>
        Start Proxy SpacetimeDB Test
      </button>

      <button onClick={exportData} style={{ marginLeft: 10 }}>
        Export Data
      </button>

      <p style={{ marginTop: 20 }}>
        Each test runs for 4 minutes: 2 minutes of latency-only pings, then 2 minutes of video/audio throughput simulation.
      </p>
    </div>
  );
}

export default App;
