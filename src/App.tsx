import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type Mode = "none" | "server" | "client";

interface DisplayInfo {
  index: number;
  width: number;
  height: number;
}

function App() {
  const [mode, setMode] = useState<Mode>("none");
  const [isActive, setIsActive] = useState(false);
  const [status, setStatus] = useState("");
  const [displays, setDisplays] = useState<DisplayInfo[]>([]);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const ctxRef = useRef<CanvasRenderingContext2D | null>(null);
  const lastFrameRef = useRef<ImageBitmap | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const isVisibleRef = useRef(true);

  // Redraw last frame when window regains focus
  const redrawLastFrame = () => {
    const canvas = canvasRef.current;
    const ctx = ctxRef.current;
    const lastFrame = lastFrameRef.current;

    if (canvas && ctx && lastFrame) {
      // Ensure canvas dimensions are correct
      if (canvas.width !== lastFrame.width || canvas.height !== lastFrame.height) {
        canvas.width = lastFrame.width;
        canvas.height = lastFrame.height;
      }
      // Redraw the last frame
      ctx.drawImage(lastFrame, 0, 0);
    }
  };

  useEffect(() => {
    // Handle visibility change (tab/window focus)
    const handleVisibilityChange = () => {
      isVisibleRef.current = !document.hidden;
      if (!document.hidden) {
        // Redraw when tab becomes visible
        redrawLastFrame();
      }
    };

    // Handle mouse enter/leave
    const handleMouseEnter = () => {
      isVisibleRef.current = true;
      redrawLastFrame();
    };

    const handleMouseLeave = () => {
      isVisibleRef.current = false;
    };

    // Handle window focus/blur
    const handleFocus = () => {
      isVisibleRef.current = true;
      redrawLastFrame();
    };

    const handleBlur = () => {
      isVisibleRef.current = false;
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('focus', handleFocus);
    window.addEventListener('blur', handleBlur);

    const canvas = canvasRef.current;
    if (canvas) {
      canvas.addEventListener('mouseenter', handleMouseEnter);
      canvas.addEventListener('mouseleave', handleMouseLeave);
    }

    const unlisten = listen<string>("screen-frame", async (event) => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      try {
        // Decode base64 to blob
        const base64Data = event.payload;
        
        // Validate base64 data is not empty
        if (!base64Data || base64Data.length < 100) {
          console.warn("Received empty or too small frame data, keeping last frame");
          return;
        }
        
        const binaryString = atob(base64Data);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
        
        // Validate JPEG signature before creating blob
        if (bytes.length < 2 || bytes[0] !== 0xFF || bytes[1] !== 0xD8) {
          console.warn("Received invalid JPEG data (missing magic bytes), keeping last frame");
          return;
        }
        
        const blob = new Blob([bytes], { type: "image/jpeg" });

        // Create ImageBitmap for better performance (with error handling)
        let imageBitmap: ImageBitmap;
        try {
          imageBitmap = await createImageBitmap(blob);
        } catch (bitmapError) {
          console.error("Failed to create ImageBitmap from received data, keeping last frame:", bitmapError);
          return; // Keep displaying last valid frame
        }

        // Initialize context once with proper settings
        if (!ctxRef.current) {
          ctxRef.current = canvas.getContext("2d", {
            alpha: false, // Better performance for opaque content
            desynchronized: true, // Allow async rendering
            willReadFrequently: false, // We only write, don't read
          });
        }

        const ctx = ctxRef.current;
        if (!ctx) {
          imageBitmap.close();
          return;
        }

        // Only resize canvas if dimensions changed
        if (canvas.width !== imageBitmap.width || canvas.height !== imageBitmap.height) {
          canvas.width = imageBitmap.width;
          canvas.height = imageBitmap.height;
        }

        // Use requestAnimationFrame for smoother rendering
        if (animationFrameRef.current) {
          cancelAnimationFrame(animationFrameRef.current);
        }

        animationFrameRef.current = requestAnimationFrame(() => {
          // Draw the ImageBitmap
          ctx.drawImage(imageBitmap, 0, 0);
          animationFrameRef.current = null;
        });

        // Clean up previous frame
        if (lastFrameRef.current && lastFrameRef.current !== imageBitmap) {
          lastFrameRef.current.close();
        }
        lastFrameRef.current = imageBitmap;
      } catch (error) {
        console.error("Failed to render frame:", error);
      }
    });

    // Load available displays
    loadDisplays();

    return () => {
      unlisten.then((fn) => fn());
      
      // Remove event listeners
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleFocus);
      window.removeEventListener('blur', handleBlur);
      
      const canvas = canvasRef.current;
      if (canvas) {
        canvas.removeEventListener('mouseenter', handleMouseEnter);
        canvas.removeEventListener('mouseleave', handleMouseLeave);
      }

      // Cancel pending animation frame
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }

      // Clean up ImageBitmap on unmount
      if (lastFrameRef.current) {
        lastFrameRef.current.close();
        lastFrameRef.current = null;
      }
      ctxRef.current = null;
    };
  }, []);

  const loadDisplays = async () => {
    try {
      const result = await invoke<DisplayInfo[]>("get_displays");
      setDisplays(result);
    } catch (error) {
      console.error("Failed to load displays:", error);
    }
  };

  const startServer = async () => {
    try {
      const result = await invoke<string>("start_server");
      setStatus(result);
      setIsActive(true);
    } catch (error) {
      setStatus(`Error: ${error}`);
    }
  };

  const stopServer = async () => {
    try {
      const result = await invoke<string>("stop_server");
      setStatus(result);
      setIsActive(false);
    } catch (error) {
      setStatus(`Error: ${error}`);
    }
  };

  const startClient = async () => {
    try {
      const result = await invoke<string>("start_client");
      setStatus(result);
      setIsActive(true);
    } catch (error) {
      setStatus(`Error: ${error}`);
    }
  };

  const stopClient = async () => {
    try {
      const result = await invoke<string>("stop_client");
      setStatus(result);
      setIsActive(false);
      
      // Clean up ImageBitmap
      if (lastFrameRef.current) {
        lastFrameRef.current.close();
        lastFrameRef.current = null;
      }
      
      // Clear canvas
      if (canvasRef.current && ctxRef.current) {
        ctxRef.current.clearRect(0, 0, canvasRef.current.width, canvasRef.current.height);
      }
    } catch (error) {
      setStatus(`Error: ${error}`);
    }
  };

  return (
    <div className="container">
      <h1>Screen Sharing - UDP Multicast</h1>
      
      {mode === "none" && (
        <div className="mode-selection">
          <h2>Chọn chế độ:</h2>
          <button onClick={() => setMode("server")} className="mode-btn">
            🖥️ Server (Giảng viên)
          </button>
          <button onClick={() => setMode("client")} className="mode-btn">
            👁️ Client (Học viên)
          </button>
        </div>
      )}

      {mode === "server" && (
        <div className="server-mode">
          <h2>🖥️ Server Mode - Chia sẻ màn hình</h2>
          
          {displays.length > 0 && (
            <div className="status">
              <strong>Màn hình khả dụng:</strong>
              {displays.map((d) => (
                <div key={d.index}>
                  Display {d.index + 1}: {d.width}x{d.height}
                </div>
              ))}
            </div>
          )}
          
          <div className="controls">
            {!isActive ? (
              <button onClick={startServer} className="start-btn">
                Bắt đầu chia sẻ
              </button>
            ) : (
              <button onClick={stopServer} className="stop-btn">
                Dừng chia sẻ
              </button>
            )}
            <button onClick={() => { setMode("none"); setIsActive(false); }} className="back-btn">
              Quay lại
            </button>
          </div>
          {isActive && (
            <div className="status-active">
              ✅ Đang chia sẻ màn hình qua UDP Multicast (239.0.0.1:9999)
              <br />
              📡 Sử dụng scrap library cho hiệu suất tối ưu
            </div>
          )}
        </div>
      )}

      {mode === "client" && (
        <div className="client-mode">
          <h2>👁️ Client Mode - Xem màn hình</h2>
          <div className="controls">
            {!isActive ? (
              <button onClick={startClient} className="start-btn">
                Kết nối
              </button>
            ) : (
              <button onClick={stopClient} className="stop-btn">
                Ngắt kết nối
              </button>
            )}
            <button onClick={() => { setMode("none"); setIsActive(false); }} className="back-btn">
              Quay lại
            </button>
          </div>
          {isActive && (
            <div className="screen-display">
              <canvas ref={canvasRef} />
            </div>
          )}
        </div>
      )}

      {status && <p className="status">{status}</p>}
    </div>
  );
}

export default App;
