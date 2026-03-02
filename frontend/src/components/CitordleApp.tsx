import { useEffect, useMemo, useRef, useState } from "react";
import type {
  CSSProperties,
  FormEvent,
  PointerEvent as ReactPointerEvent,
  TouchEvent as ReactTouchEvent
} from "react";

import CountryOutlineMap from "./CountryOutlineMap";

type StageKind = "duolingo" | "draw" | "trivia";
type Point = [number, number];
type Stroke = Point[];

type DailyGameResponse = {
  date: string;
  city_id: string;
  city_name: string;
  country: string;
  round1: {
    word_length: number;
    hints: string[];
  };
  round2: {
    prompt: string;
    country_code: string;
    map_svg: string;
    options: string[];
  };
  round3: {
    kind: StageKind;
    prompt: string;
    options: string[];
    instructions: string | null;
    guide_points: Point[] | null;
  };
};

type DailyProgress = {
  date: string;
  city_id: string;
  round1_attempts: number;
  round2_attempts: number;
  round3_attempts: number;
  round1_completed: boolean;
  round2_completed: boolean;
  round3_completed: boolean;
  completed: boolean;
};

type DailyGameEnvelope = {
  game: DailyGameResponse;
  progress: DailyProgress;
  session_token: string;
};

type LetterFeedback = {
  letter: string;
  status: "correct" | "present" | "absent";
};

type RoundOneCheckResponse = {
  correct: boolean;
  feedback: LetterFeedback[];
  progress: DailyProgress;
  session_token: string;
};

type RoundOneAttempt = {
  guess: string;
  feedback: LetterFeedback[];
};

type RoundCheckResponse = {
  correct: boolean;
  message: string;
  progress: DailyProgress;
  session_token: string;
};

type Attempts = {
  round1: number;
  round2: number;
  round3: number;
};

const KEYBOARD_ROWS = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];
const STATUS_PRIORITY: Record<LetterFeedback["status"], number> = {
  absent: 0,
  present: 1,
  correct: 2
};

const API_BASE = import.meta.env.PUBLIC_API_BASE ?? "";
const SESSION_TOKEN_STORAGE_KEY = "citordle_session_token";

function millisecondsUntilNextUtcMidnight(now: Date = new Date()): number {
  const nextUtcMidnight = Date.UTC(
    now.getUTCFullYear(),
    now.getUTCMonth(),
    now.getUTCDate() + 1,
    0,
    0,
    0,
    0
  );
  return Math.max(0, nextUtcMidnight - now.getTime());
}

function formatCountdown(milliseconds: number): string {
  const totalSeconds = Math.floor(milliseconds / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return [hours, minutes, seconds].map((value) => String(value).padStart(2, "0")).join(":");
}

async function postJson<T>(
  url: string,
  payload: Record<string, unknown>,
  token?: string
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json"
  };
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  const response = await fetch(url, {
    method: "POST",
    headers,
    body: JSON.stringify(payload)
  });

  const data = await response.json();
  if (!response.ok) {
    throw new Error(data.error ?? "Request failed.");
  }

  return data as T;
}

function clientToPoint(clientX: number, clientY: number, canvas: HTMLCanvasElement): Point {
  const rect = canvas.getBoundingClientRect();
  const x = (clientX - rect.left) / rect.width;
  const y = (clientY - rect.top) / rect.height;
  return [Math.max(0, Math.min(1, x)), Math.max(0, Math.min(1, y))];
}

function pointerToPoint(event: ReactPointerEvent<HTMLCanvasElement>, canvas: HTMLCanvasElement): Point {
  return clientToPoint(event.clientX, event.clientY, canvas);
}

function touchToPoint(event: ReactTouchEvent<HTMLCanvasElement>, canvas: HTMLCanvasElement): Point | null {
  const touch = event.touches[0] ?? event.changedTouches[0];
  if (!touch) {
    return null;
  }
  return clientToPoint(touch.clientX, touch.clientY, canvas);
}

function renderSketch(
  canvas: HTMLCanvasElement,
  strokes: Stroke[],
  activeStroke: Stroke,
  guidePoints?: Point[] | null
) {
  const context = canvas.getContext("2d");
  if (!context) {
    return;
  }

  context.clearRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = "#fffaf2";
  context.fillRect(0, 0, canvas.width, canvas.height);

  context.strokeStyle = "rgba(31, 45, 61, 0.12)";
  context.lineWidth = 1;
  context.setLineDash([8, 8]);
  context.beginPath();
  context.moveTo(canvas.width * 0.5, 0);
  context.lineTo(canvas.width * 0.5, canvas.height);
  context.moveTo(0, canvas.height * 0.5);
  context.lineTo(canvas.width, canvas.height * 0.5);
  context.stroke();
  context.setLineDash([]);

  if (guidePoints && guidePoints.length > 1) {
    context.strokeStyle = "rgba(34, 63, 88, 0.32)";
    context.lineWidth = 3;
    context.setLineDash([10, 8]);
    context.beginPath();
    context.moveTo(guidePoints[0][0] * canvas.width, guidePoints[0][1] * canvas.height);
    for (const point of guidePoints.slice(1)) {
      context.lineTo(point[0] * canvas.width, point[1] * canvas.height);
    }
    context.stroke();
    context.setLineDash([]);
  }

  const allStrokes = activeStroke.length > 1 ? [...strokes, activeStroke] : strokes;

  context.strokeStyle = "#2b3e4f";
  context.lineJoin = "round";
  context.lineCap = "round";
  context.lineWidth = 4;

  for (const stroke of allStrokes) {
    if (stroke.length === 0) {
      continue;
    }
    context.beginPath();
    context.moveTo(stroke[0][0] * canvas.width, stroke[0][1] * canvas.height);
    for (const point of stroke.slice(1)) {
      context.lineTo(point[0] * canvas.width, point[1] * canvas.height);
    }
    context.stroke();
  }
}

export default function CitordleApp() {
  const [daily, setDaily] = useState<DailyGameResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [round, setRound] = useState<1 | 2 | 3 | 4>(1);
  const [message, setMessage] = useState("Solve all three rounds to finish today.");

  const [guess, setGuess] = useState("");
  const [showHints, setShowHints] = useState(false);
  const [roundOneHistory, setRoundOneHistory] = useState<RoundOneAttempt[]>([]);
  const [keyboard, setKeyboard] = useState<Record<string, LetterFeedback["status"]>>({});

  const [drawStrokes, setDrawStrokes] = useState<Stroke[]>([]);
  const [activeStroke, setActiveStroke] = useState<Stroke>([]);
  const [isSketching, setIsSketching] = useState(false);
  const [showDrawGuide, setShowDrawGuide] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const activeStrokeRef = useRef<Stroke>([]);
  const isSketchingRef = useRef(false);

  const [attempts, setAttempts] = useState<Attempts>({
    round1: 0,
    round2: 0,
    round3: 0
  });
  const [sessionToken, setSessionToken] = useState("");
  const [resetCountdown, setResetCountdown] = useState(() =>
    formatCountdown(millisecondsUntilNextUtcMidnight())
  );

  function saveSessionToken(token: string) {
    setSessionToken(token);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(SESSION_TOKEN_STORAGE_KEY, token);
    }
  }

  function applyProgress(progress: DailyProgress) {
    setAttempts({
      round1: progress.round1_attempts,
      round2: progress.round2_attempts,
      round3: progress.round3_attempts
    });

    if (progress.completed) {
      setRound(4);
      setMessage("You already completed today's Citordle. Nice work.");
      return;
    }

    if (progress.round1_completed && progress.round2_completed) {
      setRound(3);
      setMessage("Progress restored. Finish your random round.");
      return;
    }

    if (progress.round1_completed) {
      setRound(2);
      setMessage("Progress restored. Continue with the map round.");
      return;
    }

    if (progress.round1_attempts > 0 || progress.round2_attempts > 0 || progress.round3_attempts > 0) {
      setMessage("Progress restored. Continue where you left off.");
    } else {
      setMessage("Solve all three rounds to finish today.");
    }
    setRound(1);
  }

  useEffect(() => {
    setShowDrawGuide(false);
    setDrawStrokes([]);
    setActiveStroke([]);
    activeStrokeRef.current = [];
    isSketchingRef.current = false;
    setIsSketching(false);
  }, [daily?.date, daily?.city_id]);

  useEffect(() => {
    if (typeof document === "undefined") {
      return;
    }

    const previousOverflow = document.body.style.overflow;
    if (isSketching) {
      document.body.style.overflow = "hidden";
    }

    return () => {
      document.body.style.overflow = previousOverflow;
    };
  }, [isSketching]);

  useEffect(() => {
    const updateCountdown = () => {
      setResetCountdown(formatCountdown(millisecondsUntilNextUtcMidnight()));
    };

    updateCountdown();
    const intervalId = window.setInterval(updateCountdown, 1000);
    return () => {
      window.clearInterval(intervalId);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function loadDailyGame() {
      setLoading(true);
      setError(null);
      try {
        const storedToken =
          typeof window !== "undefined"
            ? window.localStorage.getItem(SESSION_TOKEN_STORAGE_KEY) ?? ""
            : "";

        const response = await fetch(`${API_BASE}/api/daily`, {
          headers: storedToken
            ? {
                Authorization: `Bearer ${storedToken}`
              }
            : undefined
        });
        const data = (await response.json()) as DailyGameEnvelope;
        if (!response.ok) {
          throw new Error("Could not load game.");
        }
        if (!cancelled) {
          setDaily(data.game);
          saveSessionToken(data.session_token);
          applyProgress(data.progress);
        }
      } catch (requestError) {
        if (!cancelled) {
          let text = requestError instanceof Error ? requestError.message : "Could not load game data.";
          if (
            requestError instanceof TypeError ||
            /failed to fetch|fetch failed|load failed|network/i.test(String(text))
          ) {
            text =
              "Could not reach backend API. Start `just backend` and keep it running while using `bun --cwd frontend dev --host`.";
          }
          setError(text);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadDailyGame();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    const guidePoints =
      daily?.round3.kind === "draw" && attempts.round3 > 0 && showDrawGuide
        ? daily.round3.guide_points
        : null;
    renderSketch(canvas, drawStrokes, activeStroke, guidePoints);
  }, [drawStrokes, activeStroke, daily, attempts.round3, showDrawGuide]);

  const todayTotalAttempts = useMemo(
    () => attempts.round1 + attempts.round2 + attempts.round3,
    [attempts]
  );

  function appendStrokePoint(point: Point) {
    const previous = activeStrokeRef.current;
    const last = previous[previous.length - 1];
    if (!last) {
      activeStrokeRef.current = [point];
      setActiveStroke([point]);
      return;
    }

    const dx = point[0] - last[0];
    const dy = point[1] - last[1];
    if (Math.sqrt(dx * dx + dy * dy) < 0.003) {
      return;
    }

    const next = [...previous, point];
    activeStrokeRef.current = next;
    setActiveStroke(next);
  }

  function finishCurrentStroke() {
    isSketchingRef.current = false;
    setIsSketching(false);
    const finished = activeStrokeRef.current;
    setDrawStrokes((previous) => {
      if (finished.length < 2) {
        return previous;
      }
      return [...previous, finished];
    });
    activeStrokeRef.current = [];
    setActiveStroke([]);
  }

  function beginSketch(event: ReactPointerEvent<HTMLCanvasElement>) {
    if (event.pointerType === "touch") {
      return;
    }

    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    event.preventDefault();
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // Pointer capture is not always available on mobile browsers.
    }
    isSketchingRef.current = true;
    setIsSketching(true);
    const first = pointerToPoint(event, canvas);
    activeStrokeRef.current = [first];
    setActiveStroke([first]);
  }

  function continueSketch(event: ReactPointerEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    if (!canvas || !isSketchingRef.current || event.pointerType === "touch") {
      return;
    }
    event.preventDefault();
    appendStrokePoint(pointerToPoint(event, canvas));
  }

  function endSketch(event: ReactPointerEvent<HTMLCanvasElement>) {
    if (!isSketchingRef.current || event.pointerType === "touch") {
      return;
    }
    event.preventDefault();
    try {
      event.currentTarget.releasePointerCapture(event.pointerId);
    } catch {
      // Ignore on browsers without pointer capture support.
    }
    finishCurrentStroke();
  }

  function beginSketchTouch(event: ReactTouchEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    const point = touchToPoint(event, canvas);
    if (!point) {
      return;
    }
    event.preventDefault();
    isSketchingRef.current = true;
    setIsSketching(true);
    activeStrokeRef.current = [point];
    setActiveStroke([point]);
  }

  function continueSketchTouch(event: ReactTouchEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    if (!canvas || !isSketchingRef.current) {
      return;
    }
    const point = touchToPoint(event, canvas);
    if (!point) {
      return;
    }
    event.preventDefault();
    appendStrokePoint(point);
  }

  function endSketchTouch(event: ReactTouchEvent<HTMLCanvasElement>) {
    if (!isSketchingRef.current) {
      return;
    }
    event.preventDefault();
    finishCurrentStroke();
  }

  function clearSketch() {
    isSketchingRef.current = false;
    activeStrokeRef.current = [];
    setIsSketching(false);
    setDrawStrokes([]);
    setActiveStroke([]);
  }

  async function onRoundOneSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!daily) {
      return;
    }

    const normalizedGuess = guess.replace(/[^A-Za-z]/g, "").toUpperCase();
    if (!normalizedGuess) {
      setMessage("Enter a word guess using letters.");
      return;
    }

    try {
      const result = await postJson<RoundOneCheckResponse>(
        `${API_BASE}/api/check/round1`,
        {
          guess: normalizedGuess
        },
        sessionToken
      );
      setRoundOneHistory((prev) => [
        ...prev,
        {
          guess: normalizedGuess,
          feedback: result.feedback
        }
      ]);

      setKeyboard((prev) => {
        const next = { ...prev };
        for (const item of result.feedback) {
          const current = next[item.letter];
          if (!current || STATUS_PRIORITY[item.status] > STATUS_PRIORITY[current]) {
            next[item.letter] = item.status;
          }
        }
        return next;
      });

      saveSessionToken(result.session_token);
      applyProgress(result.progress);

      setGuess("");

      if (result.correct) {
        setMessage("Round 1 complete. Geography round unlocked.");
      } else {
        setMessage("Keep going. Green is exact, yellow is in the word, gray is absent.");
      }
    } catch (requestError) {
      const text = requestError instanceof Error ? requestError.message : "Invalid guess.";
      setMessage(text);
    }
  }

  async function onRoundTwoAnswer(answer: string) {
    try {
      const result = await postJson<RoundCheckResponse>(
        `${API_BASE}/api/check/round2`,
        {
          answer
        },
        sessionToken
      );

      saveSessionToken(result.session_token);
      applyProgress(result.progress);
      setMessage(result.message);
    } catch {
      setMessage("Could not validate geography answer.");
    }
  }

  async function onRoundThreeAnswer(answer: string) {
    try {
      const result = await postJson<RoundCheckResponse>(
        `${API_BASE}/api/check/round3`,
        {
          answer
        },
        sessionToken
      );

      saveSessionToken(result.session_token);
      applyProgress(result.progress);
      setMessage(result.message);
    } catch {
      setMessage("Could not validate stage 3 answer.");
    }
  }

  async function onDrawSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      const payloadStrokes =
        activeStrokeRef.current.length >= 2
          ? [...drawStrokes, activeStrokeRef.current]
          : drawStrokes;

      const result = await postJson<RoundCheckResponse>(
        `${API_BASE}/api/check/round3`,
        {
          strokes: payloadStrokes
        },
        sessionToken
      );

      saveSessionToken(result.session_token);
      applyProgress(result.progress);
      setMessage(result.message);
    } catch {
      setMessage("Could not submit drawing stage.");
    }
  }

  if (loading) {
    return (
      <main className="shell">
        <section className="panel">
          <h1>Citordle</h1>
          <p>Loading your city challenge...</p>
        </section>
      </main>
    );
  }

  if (error || !daily) {
    return (
      <main className="shell">
        <section className="panel">
          <h1>Citordle</h1>
          <p>{error ?? "Could not load the game."}</p>
        </section>
      </main>
    );
  }

  const stats = [
    { label: "City Word", value: attempts.round1 },
    { label: "Map Round", value: attempts.round2 },
    { label: "Random Round", value: attempts.round3 }
  ];
  const maxAttempt = Math.max(1, ...stats.map((item) => item.value));
  const liveGuess = guess.replace(/[^A-Za-z]/g, "").toUpperCase().slice(0, daily.round1.word_length);
  const boardRows = Math.max(6, roundOneHistory.length + (round === 1 ? 1 : 0));
  const boardTemplateStyle: CSSProperties = {
    gridTemplateColumns: `repeat(${daily.round1.word_length}, var(--tile-size))`
  };

  return (
    <main className="shell">
      <section className="panel">
        <header className="hero">
          <p className="eyebrow">Daily City Puzzle</p>
          <h1>Citordle</h1>
          <p>
            <strong>{daily.city_name}</strong>, {daily.country} · {daily.date}
          </p>
        </header>

        <p className="status">{message}</p>

        {round === 1 && (
          <section className="round">
            <h2>Round 1: City Word</h2>
            <p>Guess the {daily.round1.word_length}-letter word linked to this city.</p>
            <button className="secondary hint-toggle" type="button" onClick={() => setShowHints((prev) => !prev)}>
              {showHints ? "Hide hints" : "Show hints"}
            </button>
            {showHints && (
              <ul className="hints">
                {daily.round1.hints.map((hint) => (
                  <li key={hint}>{hint}</li>
                ))}
              </ul>
            )}

            <div className="wordle-board" aria-live="polite">
              {roundOneHistory.map((attempt, index) => (
                <div className="board-row" style={boardTemplateStyle} key={`${attempt.guess}-${index}`}>
                  {attempt.feedback.map((item, tileIndex) => (
                    <div key={`${item.letter}-${tileIndex}`} className={`board-tile ${item.status}`}>
                      {item.letter}
                    </div>
                  ))}
                </div>
              ))}

              {round === 1 && (
                <div className="board-row current-row" style={boardTemplateStyle}>
                  {Array.from({ length: daily.round1.word_length }, (_, index) => {
                    const letter = liveGuess[index] ?? "";
                    return (
                      <div key={`current-${index}`} className={`board-tile ${letter ? "filled" : "empty"}`}>
                        {letter}
                      </div>
                    );
                  })}
                </div>
              )}

              {Array.from({ length: Math.max(0, boardRows - roundOneHistory.length - (round === 1 ? 1 : 0)) }).map(
                (_, index) => (
                  <div className="board-row" style={boardTemplateStyle} key={`empty-row-${index}`}>
                    {Array.from({ length: daily.round1.word_length }, (_, tileIndex) => (
                      <div key={`empty-${index}-${tileIndex}`} className="board-tile empty" />
                    ))}
                  </div>
                )
              )}
            </div>

            <form className="guess-form" onSubmit={onRoundOneSubmit}>
              <input
                type="text"
                value={guess}
                maxLength={daily.round1.word_length}
                onChange={(event) =>
                  setGuess(
                    event.target.value
                      .replace(/[^A-Za-z]/g, "")
                      .toUpperCase()
                      .slice(0, daily.round1.word_length)
                  )
                }
                placeholder="Your guess"
                aria-label="Word guess"
              />
              <button type="submit">Check</button>
            </form>

            <div className="keyboard" aria-label="Word clues keyboard">
              {KEYBOARD_ROWS.map((row) => (
                <div className="keyboard-row" key={row}>
                  {row.split("").map((letter) => (
                    <span key={letter} className={`key ${keyboard[letter] ?? "unused"}`}>
                      {letter}
                    </span>
                  ))}
                </div>
              ))}
            </div>
          </section>
        )}

        {round === 2 && (
          <section className="round">
            <h2>Round 2: Map Read</h2>
            <p>{daily.round2.prompt}</p>

            <div className="map-card">
              <CountryOutlineMap
                countryCode={daily.round2.country_code}
                countryName={daily.country}
                fallbackPath={daily.round2.map_svg}
              />
            </div>

            <div className="options-grid">
              {daily.round2.options.map((option) => (
                <button key={option} type="button" onClick={() => onRoundTwoAnswer(option)}>
                  {option}
                </button>
              ))}
            </div>
          </section>
        )}

        {round === 3 && (
          <section className="round">
            <h2>Round 3: Random Challenge</h2>
            <p>{daily.round3.prompt}</p>
            {daily.round3.instructions && <p className="instruction">{daily.round3.instructions}</p>}

            {daily.round3.kind === "draw" ? (
              <form className="draw-form" onSubmit={onDrawSubmit}>
                <p className="draw-tip">
                  {attempts.round3 > 0
                    ? "Hint available: use the button below to show or hide the landmark guide."
                    : "First try is guide-free. Submit once to unlock the hint button."}
                </p>
                <div className="sketch-area">
                  <div className="sketch-canvas-wrap">
                    <canvas
                      ref={canvasRef}
                      className="sketch-canvas"
                      width={520}
                      height={320}
                      onPointerDown={beginSketch}
                      onPointerMove={continueSketch}
                      onPointerUp={endSketch}
                      onPointerLeave={endSketch}
                      onPointerCancel={endSketch}
                      onTouchStart={beginSketchTouch}
                      onTouchMove={continueSketchTouch}
                      onTouchEnd={endSketchTouch}
                      onTouchCancel={endSketchTouch}
                    />
                  </div>
                  <div className="draw-actions">
                    <button type="button" className="secondary" onClick={clearSketch}>
                      Clear canvas
                    </button>
                    <button
                      type="button"
                      className="secondary"
                      disabled={attempts.round3 === 0}
                      onClick={() => setShowDrawGuide((prev) => !prev)}
                    >
                      {showDrawGuide ? "Hide hint" : "Show hint"}
                    </button>
                    <span>{drawStrokes.length} stroke(s) captured</span>
                  </div>
                </div>
                <button type="submit" disabled={drawStrokes.length === 0}>
                  Submit drawing stage
                </button>
              </form>
            ) : (
              <div className="options-grid">
                {daily.round3.options.map((option) => (
                  <button key={option} type="button" onClick={() => onRoundThreeAnswer(option)}>
                    {option}
                  </button>
                ))}
              </div>
            )}
          </section>
        )}

        {round === 4 && (
          <section className="round done">
            <h2>Congrats, you did it!</h2>
            <p>You solved all stages for today in {todayTotalAttempts} total tries.</p>
            <p className="next-reset">Next city in {resetCountdown} (UTC).</p>

            <div className="distribution">
              {stats.map((item) => (
                <div key={item.label} className="distribution-row">
                  <span>{item.label}</span>
                  <div className="bar-wrap">
                    <div className="bar" style={{ width: `${(item.value / maxAttempt) * 100}%` }} />
                  </div>
                  <strong>{item.value}</strong>
                </div>
              ))}
            </div>
          </section>
        )}
      </section>
    </main>
  );
}
