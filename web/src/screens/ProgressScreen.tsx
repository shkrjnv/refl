import { useEffect, useMemo, useRef, useState } from "react";

import { Card, Disclosure, ProgressBar, StatusMeta } from "../components/ui";
import { useTranslation } from "../i18n";
import type { PrototypeContent } from "../prototype/content";
import {
  averageElapsedDays,
  domainBreakdown,
  kcProgress,
  levelBreakdown,
  memoryRetention,
  misconceptionCureRate,
  misconceptionProgress,
  parseLearnerState,
  statsSummary,
} from "../prototype/progress";

export interface ProgressScreenProps {
  content: PrototypeContent;
  stateJson: string | null;
}

const CEFR_TIERS = ["A1", "A2", "B1", "B2"] as const;

const DOMAIN_KEYS: Record<string, string> = {
  grammar: "domain-grammar",
  phonology: "domain-phonology",
  orthography: "domain-orthography",
  discourse: "domain-discourse",
  "reading-skill": "domain-reading",
  "listening-skill": "domain-listening",
  "writing-skill": "domain-writing",
};

function domainLabel(domain: string, t: (key: string) => string): string {
  return t(DOMAIN_KEYS[domain] ?? domain);
}

function ForgettingCurve({
  stability,
  elapsed,
}: {
  stability: number;
  elapsed: number;
}) {
  const { t } = useTranslation();
  const safeStability = Math.max(0.5, stability);
  const maxDays = Math.max(
    30,
    Math.ceil(safeStability * 3.5),
    Math.ceil(elapsed) + 1,
  );
  const [selected, setSelected] = useState(() =>
    Math.max(0, Math.min(maxDays, Math.round(elapsed))),
  );
  const [rubberOffset, setRubberOffset] = useState(0);
  const [rubberSnapping, setRubberSnapping] = useState(false);
  const rubberPointerRef = useRef<number | null>(null);
  const rubberTimerRef = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (rubberTimerRef.current !== null) {
        window.clearTimeout(rubberTimerRef.current);
      }
    },
    [],
  );

  const settleRubber = () => {
    rubberPointerRef.current = null;
    setRubberOffset(0);
    setRubberSnapping(true);
    if (rubberTimerRef.current !== null) {
      window.clearTimeout(rubberTimerRef.current);
    }
    rubberTimerRef.current = window.setTimeout(() => {
      rubberTimerRef.current = null;
      setRubberSnapping(false);
    }, 340);
  };
  const chartWidth = 560;
  const chartHeight = 228;
  const left = 42;
  const right = 14;
  const top = 14;
  const bottom = 46;
  const plotWidth = chartWidth - left - right;
  const plotHeight = chartHeight - top - bottom;
  const xAt = (day: number) => left + (day / maxDays) * plotWidth;
  const yAt = (ratio: number) => top + (1 - ratio) * plotHeight;
  const points = Array.from({ length: 73 }, (_, index) => {
    const day = (index / 72) * maxDays;
    const ratio = 1 / (1 + day / (9 * safeStability));
    return { day, ratio, x: xAt(day), y: yAt(ratio) };
  });
  const path = points
    .map((point, index) => `${index === 0 ? "M" : "L"}${point.x.toFixed(1)} ${point.y.toFixed(1)}`)
    .join(" ");
  const area = `${path} L${xAt(maxDays)} ${yAt(0)} L${xAt(0)} ${yAt(0)} Z`;
  const selectedRatio = 1 / (1 + selected / (9 * safeStability));

  return (
    <div className="forgetting-curve">
      <div className="curve-particles" aria-hidden="true" />
      <svg
        viewBox={`0 0 ${chartWidth} ${chartHeight}`}
        role="img"
        aria-label={t("progress-curve-title")}
      >
        <g className="chart-grid">
          {[0, 0.5, 0.9, 1].map((ratio) => (
            <line
              key={ratio}
              x1={left}
              x2={chartWidth - right}
              y1={yAt(ratio)}
              y2={yAt(ratio)}
            />
          ))}
        </g>
        <path className="chart-area" d={area} />
        <path className="chart-curve" d={path} />
        <line
          className="chart-target"
          x1={left}
          x2={chartWidth - right}
          y1={yAt(0.9)}
          y2={yAt(0.9)}
        />
        <circle
          className="chart-marker"
          cx={xAt(selected)}
          cy={yAt(selectedRatio)}
          r="5"
        />
        <g className="chart-labels">
          <text x={left - 9} y={yAt(1) + 4} textAnchor="end">
            100%
          </text>
          <text x={left - 9} y={yAt(0.9) + 4} textAnchor="end">
            90%
          </text>
          <text x={left - 9} y={yAt(0.5) + 4} textAnchor="end">
            50%
          </text>
          <text x={left - 9} y={yAt(0) + 4} textAnchor="end">
            0%
          </text>
          <text x={left} y={chartHeight - 24}>
            0
          </text>
          <text x={xAt(maxDays / 2)} y={chartHeight - 24} textAnchor="middle">
            {Math.round(maxDays / 2)}
          </text>
          <text x={xAt(maxDays)} y={chartHeight - 24} textAnchor="end">
            {maxDays}
          </text>
        </g>
        <text
          className="chart-axis-title"
          x={left + plotWidth / 2}
          y={chartHeight - 6}
          textAnchor="middle"
        >
          {t("progress-days")}
        </text>
      </svg>
      <div className="curve-control">
        <label>
          <span>{t("progress-curve-time", { days: selected })}</span>
          <input
            className={`curve-range${rubberSnapping ? " snapping" : ""}`}
            type="range"
            min={0}
            max={maxDays}
            step={1}
            value={selected}
            style={{ transform: `translateX(${rubberOffset}px)` }}
            aria-label={t("progress-curve-scrub")}
            onChange={(event) => setSelected(Number(event.target.value))}
            onPointerDown={(event) => {
              rubberPointerRef.current = event.pointerId;
              setRubberSnapping(false);
              event.currentTarget.setPointerCapture?.(event.pointerId);
            }}
            onPointerMove={(event) => {
              if (rubberPointerRef.current !== event.pointerId) {
                return;
              }
              const bounds = event.currentTarget.getBoundingClientRect();
              let overscroll = 0;
              if (event.clientX < bounds.left) {
                overscroll = (event.clientX - bounds.left) * 0.28;
              } else if (event.clientX > bounds.right) {
                overscroll = (event.clientX - bounds.right) * 0.28;
              }
              setRubberOffset(Math.max(-18, Math.min(18, overscroll)));
            }}
            onPointerUp={settleRubber}
            onPointerCancel={settleRubber}
          />
        </label>
        <strong>
          {t("progress-curve-retention", {
            percent: Math.round(selectedRatio * 100),
          })}
        </strong>
      </div>
    </div>
  );
}

function IsometricMastery({
  levels,
}: {
  levels: ReturnType<typeof levelBreakdown>;
}) {
  const { t } = useTranslation();
  const [hovered, setHovered] = useState<number | null>(null);
  const width = 560;
  const height = 300;
  const baseY = 252;
  const halfWidth = 31;
  const halfDepth = 13;
  const step =
    levels.length === 0
      ? 100
      : Math.min(108, (width - 100) / Math.max(1, levels.length));
  const startX = width / 2 - (step * (levels.length - 1)) / 2;
  if (levels.length === 0) {
    return <p className="chart-empty">{t("progress-chart-empty")}</p>;
  }
  const hoveredLevel = hovered === null ? null : levels[hovered] ?? null;
  return (
    <div className="iso-chart-wrap">
      <svg
        className="mastery-chart iso-mastery-chart"
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label={t("progress-mastery-chart")}
      >
        <line
          className="iso-ground"
          x1={24}
          x2={width - 24}
          y1={baseY + halfDepth}
          y2={baseY + halfDepth}
        />
        {levels.map((level, index) => {
          const cx = startX + index * step;
          const blockHeight = Math.max(18, (level.percent / 100) * 166);
          const topY = baseY - blockHeight;
          const top = { x: cx, y: topY - halfDepth };
          const right = { x: cx + halfWidth, y: topY };
          const bottom = { x: cx, y: topY + halfDepth };
          const left = { x: cx - halfWidth, y: topY };
          const baseLeft = { x: cx - halfWidth, y: baseY };
          const baseBottom = { x: cx, y: baseY + halfDepth };
          const baseRight = { x: cx + halfWidth, y: baseY };
          const leftFace = `${left.x},${left.y} ${bottom.x},${bottom.y} ${baseBottom.x},${baseBottom.y} ${baseLeft.x},${baseLeft.y}`;
          const rightFace = `${right.x},${right.y} ${bottom.x},${bottom.y} ${baseBottom.x},${baseBottom.y} ${baseRight.x},${baseRight.y}`;
          const topFace = `${top.x},${top.y} ${right.x},${right.y} ${bottom.x},${bottom.y} ${left.x},${left.y}`;
          const seams = [0.25, 0.5, 0.75].map((ratio) => {
            const y = baseY - blockHeight * ratio;
            return (
              <path
                key={ratio}
                className="iso-seam"
                d={`M${left.x} ${y} L${bottom.x} ${y + halfDepth} L${right.x} ${y}`}
              />
            );
          });
          return (
            <g
              key={level.level}
              className={`iso-tier${hovered === index ? " hovered" : ""}`}
              tabIndex={0}
              role="button"
              aria-label={t("progress-iso-tooltip", {
                level: level.level,
                mastered: level.mastered,
                total: level.total,
                percent: level.percent,
              })}
              onPointerEnter={() => setHovered(index)}
              onPointerLeave={() => setHovered(null)}
              onFocus={() => setHovered(index)}
              onBlur={() => setHovered(null)}
            >
              <g
                className="iso-body"
                style={{ animationDelay: `${index * 70}ms` }}
              >
                <polygon className="iso-left" points={leftFace} />
                <polygon className="iso-right" points={rightFace} />
                <polygon className="iso-top" points={topFace} />
                {seams}
              </g>
              <text className="iso-value" x={cx} y={topY - 20} textAnchor="middle">
                {level.percent}%
              </text>
              <text className="iso-level" x={cx} y={baseY + 34} textAnchor="middle">
                {level.level}
              </text>
            </g>
          );
        })}
      </svg>
      {hoveredLevel ? (
        <div className="iso-tooltip" role="status">
          <span>
            {t("progress-iso-tooltip", {
              level: hoveredLevel.level,
              mastered: hoveredLevel.mastered,
              total: hoveredLevel.total,
              percent: hoveredLevel.percent,
            })}
          </span>
        </div>
      ) : null}
    </div>
  );
}

function CalibrationChart() {
  const { t } = useTranslation();
  const width = 320;
  const height = 230;
  const pad = 32;
  return (
    <div className="calibration-chart-wrap">
      <svg
        className="calibration-chart"
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label={t("stats-calibration")}
      >
        <g className="chart-grid">
          {[0, 0.25, 0.5, 0.75, 1].map((ratio) => (
            <g key={ratio}>
              <line
                x1={pad}
                x2={width - pad}
                y1={pad + (height - pad * 2) * ratio}
                y2={pad + (height - pad * 2) * ratio}
              />
              <line
                x1={pad + (width - pad * 2) * ratio}
                x2={pad + (width - pad * 2) * ratio}
                y1={pad}
                y2={height - pad}
              />
            </g>
          ))}
        </g>
        <line
          className="chart-target calibration-target"
          x1={pad}
          y1={height - pad}
          x2={width - pad}
          y2={pad}
        />
        <g className="chart-labels">
          <text x={pad} y={height - 10}>
            {t("progress-predicted")}
          </text>
          <text x={4} y={pad + 4} transform={`rotate(-90 4 ${pad + 4})`}>
            {t("progress-actual")}
          </text>
        </g>
      </svg>
      <p className="chart-empty">{t("progress-calibration-empty")}</p>
    </div>
  );
}

export function ProgressScreen({ content, stateJson }: ProgressScreenProps) {
  const { t, locale } = useTranslation();
  const state = useMemo(() => parseLearnerState(stateJson), [stateJson]);
  const progress = useMemo(
    () => kcProgress(content, state, locale),
    [content, state, locale],
  );
  const stats = statsSummary(progress);
  const retention = memoryRetention(progress);
  const elapsed = averageElapsedDays(progress);
  const levels = useMemo(() => {
    const observed = levelBreakdown(progress);
    const byLevel = new Map(observed.map((entry) => [entry.level, entry]));
    const base = CEFR_TIERS.map(
      (level) =>
        byLevel.get(level) ?? {
          level,
          total: 0,
          mastered: 0,
          percent: 0,
        },
    );
    return [
      ...base,
      ...observed.filter(
        (entry) => !CEFR_TIERS.includes(entry.level as (typeof CEFR_TIERS)[number]),
      ),
    ];
  }, [progress]);
  const domains = domainBreakdown(progress);
  const misconceptions = misconceptionProgress(content, state, locale);
  const cureRate = misconceptionCureRate(misconceptions);
  const activeMisconceptions = misconceptions.filter(
    (entry) => entry.status === "active",
  ).length;
  const curedMisconceptions = misconceptions.filter(
    (entry) => entry.status === "cured",
  ).length;

  return (
    <div className="stats-screen progress-screen">
      <header className="page-heading">
        <div>
          <p className="eyebrow">{t("stats-eyebrow")}</p>
          <h1>{t("stats-title")}</h1>
        </div>
        <StatusMeta tone="neutral">{t("stats-articles")}</StatusMeta>
      </header>

      <div className="progress-dashboard">
        <Card tone="plain" className="progress-card progress-fsrs-card">
          <div className="progress-card-heading">
            <div>
              <p className="eyebrow">{t("progress-memory-kicker")}</p>
              <h2>{t("progress-memory-title")}</h2>
            </div>
            <StatusMeta tone="accent" dot>
              {t("progress-fsrs-badge")}
            </StatusMeta>
          </div>
          <p className="progress-note">{t("progress-fsrs-note")}</p>
          <div className="fsrs-metrics">
            <div>
              <span>{t("progress-stability")}</span>
              <strong>
                {stats.averageStability > 0
                  ? Math.round(stats.averageStability)
                  : 0}
                <small>{t("progress-days-short")}</small>
              </strong>
            </div>
            <div>
              <span>{t("progress-difficulty")}</span>
              <strong>
                {stats.averageDifficulty > 0
                  ? stats.averageDifficulty.toFixed(1)
                  : 0}
              </strong>
            </div>
            <div>
              <span>{t("progress-retention")}</span>
              <strong>{retention}%</strong>
            </div>
            <div>
              <span>{t("progress-elapsed")}</span>
              <strong>{Math.round(elapsed)}</strong>
            </div>
          </div>
          <div className="progress-chart-heading">
            <h3>{t("progress-curve-title")}</h3>
            <span>{t("progress-target", { percent: 90 })}</span>
          </div>
          <ForgettingCurve
            stability={stats.averageStability}
            elapsed={elapsed}
          />
          <Disclosure summary={t("stats-where")}>
            <p>{t("stats-where-memory")}</p>
          </Disclosure>
        </Card>

        <Card tone="plain" className="progress-card progress-mastery-card">
          <div className="progress-card-heading">
            <div>
              <p className="eyebrow">{t("progress-mastery-kicker")}</p>
              <h2>{t("progress-mastery-title")}</h2>
            </div>
            <StatusMeta tone="success" dot>
              {t("stats-mastered")}
            </StatusMeta>
          </div>
          <IsometricMastery levels={levels} />
          <p className="progress-note">
            {t("stats-mastered-note", { total: progress.length })}
          </p>
          <Disclosure summary={t("stats-where")}>
            <p>{t("stats-where-mastered")}</p>
          </Disclosure>
        </Card>

        <Card tone="plain" className="progress-card progress-skills-card">
          <div className="progress-card-heading">
            <div>
              <p className="eyebrow">{t("progress-skills-kicker")}</p>
              <h2>{t("progress-skills-title")}</h2>
            </div>
          </div>
          {domains.length === 0 ? (
            <p className="chart-empty">{t("progress-chart-empty")}</p>
          ) : (
            <div className="skill-list">
              {domains.map((domain) => (
                <div className="skill-row" key={domain.domain}>
                  <div>
                    <strong>{domainLabel(domain.domain, t)}</strong>
                    <span>
                      {domain.mastered}/{domain.total} · {domain.percent}%
                    </span>
                  </div>
                  <ProgressBar
                    value={domain.percent}
                    tone={domain.mastered > 0 ? "green" : "blue"}
                    label={domainLabel(domain.domain, t)}
                  />
                </div>
              ))}
            </div>
          )}
        </Card>

        <Card tone="plain" className="progress-card progress-misconceptions-card">
          <div className="progress-card-heading">
            <div>
              <p className="eyebrow">{t("progress-misconceptions-kicker")}</p>
              <h2>{t("progress-misconceptions-title")}</h2>
            </div>
            <StatusMeta tone={activeMisconceptions > 0 ? "danger" : "success"} dot>
              {activeMisconceptions}
            </StatusMeta>
          </div>
          <div className="misconception-summary">
            <span>{t("progress-active", { count: activeMisconceptions })}</span>
            <span>{t("progress-cured", { count: curedMisconceptions })}</span>
            <strong>
              {cureRate === null
                ? t("progress-cure-empty")
                : t("progress-cure-rate", { percent: cureRate })}
            </strong>
          </div>
          {misconceptions.length === 0 ? (
            <p className="chart-empty">{t("progress-misconceptions-empty")}</p>
          ) : (
            <div className="misconception-list">
              {misconceptions.map((entry) => (
                <div className="misconception-row" key={entry.id}>
                  <div className="misconception-topline">
                    <strong>{entry.title}</strong>
                    <StatusMeta
                      tone={
                        entry.status === "active"
                          ? "danger"
                          : entry.status === "cured"
                            ? "success"
                            : "neutral"
                      }
                      dot
                    >
                      {t(`progress-status-${entry.status}`)}
                    </StatusMeta>
                  </div>
                  <ProgressBar
                    value={Math.round(entry.strength * 100)}
                    tone={entry.status === "active" ? "coral" : "green"}
                    label={entry.title}
                  />
                  <span>
                    {t("progress-manifested", {
                      count: entry.manifested,
                      percent: Math.round(entry.strength * 100),
                    })}
                  </span>
                </div>
              ))}
            </div>
          )}
        </Card>

        <Card tone="plain" className="progress-card progress-calibration-card">
          <div className="progress-card-heading">
            <div>
              <p className="eyebrow">{t("progress-calibration-kicker")}</p>
              <h2>{t("stats-calibration")}</h2>
            </div>
            <StatusMeta tone={stats.calibration === null ? "neutral" : "success"}>
              {stats.calibration === null ? "—" : `%`}
            </StatusMeta>
          </div>
          <CalibrationChart />
          <p className="progress-note">{t("progress-calibration-note")}</p>
          <Disclosure summary={t("stats-where")}>
            <p>{t("stats-where-calibration")}</p>
          </Disclosure>
        </Card>

        <Card tone="tinted" className="kc-table progress-kc-card">
          <div className="section-heading">
            <div>
              <p className="eyebrow">{t("stats-kc-eyebrow")}</p>
              <h2>{t("stats-kc-title")}</h2>
            </div>
          </div>
          <div className="kc-table-list">
            {progress.map((kc) => (
              <div className="kc-row" key={kc.kcId}>
                <div className="kc-row-title">
                  <strong>{kc.title}</strong>
                  <span>
                    {kc.level} ·{" "}
                    {kc.mastered
                      ? t("today-kc-mastered")
                      : kc.observations > 0
                        ? t("today-kc-progress")
                        : t("today-kc-new")}
                  </span>
                </div>
                <div className="kc-row-progress">
                  <ProgressBar
                    value={kc.percent}
                    tone={kc.mastered ? "green" : "blue"}
                  />
                  <span>{kc.percent}%</span>
                </div>
              </div>
            ))}
          </div>
        </Card>
      </div>
    </div>
  );
}
