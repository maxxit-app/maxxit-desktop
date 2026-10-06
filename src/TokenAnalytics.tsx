import { useState } from "react";
import { Activity, Download } from "lucide-react";
import { tokenSeries, tokenSummary, type TokenDay } from "./analytics";

const compact = (value: number) =>
  new Intl.NumberFormat(undefined, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
const dateLabel = (date: string, full = false) =>
  new Date(`${date}T00:00:00Z`).toLocaleDateString(undefined, {
    timeZone: "UTC",
    month: full ? "long" : "short",
    day: "numeric",
    ...(full ? { year: "numeric" } : {}),
  });

export function TokenAnalytics({ records }: { records: TokenDay[] }) {
  const [range, setRange] = useState(14);
  const [selected, setSelected] = useState<string | null>(null);
  const series = tokenSeries(records, range);
  const summary = tokenSummary(series);
  const day =
    series.find((item) => item.startDate === selected) ??
    series[series.length - 1];
  const ceiling = summary.peak?.tokens
    ? Math.pow(10, Math.floor(Math.log10(summary.peak.tokens))) *
      Math.ceil(
        summary.peak.tokens /
          Math.pow(10, Math.floor(Math.log10(summary.peak.tokens))),
      )
    : 1;
  const exportCsv = () => {
    const csv =
      "date_utc,observed_tokens\n" +
      series.map((item) => `${item.startDate},${item.tokens ?? ""}`).join("\n");
    const url = URL.createObjectURL(
      new Blob([csv], { type: "text/csv;charset=utf-8" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = `maxxit-codex-${range}-days.csv`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  };
  return (
    <section
      className="card token-analytics"
      aria-label="Codex token analytics"
    >
      <div className="token-heading">
        <div>
          <span className="eyebrow">LOCAL ACTIVITY</span>
          <h2>How you use Codex</h2>
          <p className="muted">Daily token increments observed on this Mac.</p>
        </div>
        <div className="token-range" aria-label="Chart date range">
          {[7, 14, 30].map((days) => (
            <button
              key={days}
              aria-pressed={range === days}
              onClick={() => {
                setRange(days);
                setSelected(null);
              }}
            >
              {days} days
            </button>
          ))}
        </div>
      </div>
      <div className="token-stats">
        <div>
          <span>Observed tokens</span>
          <strong>
            {summary.observedDays ? compact(summary.total) : "Unavailable"}
          </strong>
          <small>
            {dateLabel(series[0].startDate)} to{" "}
            {dateLabel(series[series.length - 1].startDate)}
          </small>
        </div>
        <div>
          <span>Average per observed day</span>
          <strong>
            {summary.average === null
              ? "Unavailable"
              : compact(summary.average)}
          </strong>
          <small>
            {summary.observedDays} of {range} days have records
          </small>
        </div>
        <div>
          <span>Busiest observed day</span>
          <strong>
            {summary.peak ? compact(summary.peak.tokens ?? 0) : "Unavailable"}
          </strong>
          <small>
            {summary.peak
              ? dateLabel(summary.peak.startDate)
              : "Waiting for usage"}
          </small>
        </div>
      </div>
      {summary.observedDays ? (
        <div className="token-explorer">
          <div className="token-chart-scroll">
            <div
              className="token-plot"
              style={{
                minWidth: `${range * 36 + 52}px`,
                maxWidth: `${range * 64 + 52}px`,
              }}
            >
              <div className="token-axis" aria-hidden="true">
                {[1, 0.75, 0.5, 0.25, 0].map((tick) => (
                  <span key={tick}>{compact(ceiling * tick)}</span>
                ))}
              </div>
              <div className="token-chart-body">
                <div className="token-grid" aria-hidden="true">
                  {[0, 1, 2, 3, 4].map((tick) => (
                    <i key={tick} />
                  ))}
                </div>
                <div
                  className="token-columns"
                  style={{
                    gridTemplateColumns: `repeat(${range}, minmax(0, 1fr))`,
                  }}
                >
                  {series.map((item) => (
                    <button
                      key={item.startDate}
                      className={`token-column ${day.startDate === item.startDate ? "selected" : ""} ${item.tokens === null ? "unobserved" : ""}`}
                      aria-pressed={day.startDate === item.startDate}
                      aria-label={`${dateLabel(item.startDate, true)}: ${item.tokens === null ? "No observations" : `${item.tokens.toLocaleString()} observed tokens`}`}
                      onMouseEnter={() => setSelected(item.startDate)}
                      onFocus={() => setSelected(item.startDate)}
                      onClick={() => setSelected(item.startDate)}
                    >
                      <span className="token-bar-area">
                        <span
                          className="token-bar"
                          style={{
                            height:
                              item.tokens === null
                                ? "0"
                                : `${Math.max(item.tokens > 0 ? 1 : 0, (item.tokens / ceiling) * 100)}%`,
                          }}
                        />
                        {item.tokens === null && (
                          <span className="token-missing">·</span>
                        )}
                      </span>
                      <span className="token-date">
                        {new Date(`${item.startDate}T00:00:00Z`).getUTCDate()}
                        <small>
                          {new Date(
                            `${item.startDate}T00:00:00Z`,
                          ).toLocaleDateString(undefined, {
                            month: "short",
                            timeZone: "UTC",
                          })}
                        </small>
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>
          <aside className="token-detail" aria-live="polite">
            <span className="muted">{dateLabel(day.startDate, true)}</span>
            <strong>
              {day.tokens === null ? "No observations" : compact(day.tokens)}
            </strong>
            <p>
              {day.tokens === null
                ? "No local records were found for this day. Usage may be missing."
                : `${day.tokens.toLocaleString()} observed tokens${day.startDate === series[series.length - 1].startDate ? ". Today is still in progress." : "."}`}
            </p>
            <div className="token-detail-note">
              <span>Reading the chart</span>
              <p>
                Hover or select a day for its exact total. Dots mark days
                without records.
              </p>
            </div>
          </aside>
        </div>
      ) : (
        <div className="empty-state">
          <Activity size={28} />
          <h3>No observations in this period.</h3>
          <p>
            Use Codex, then refresh usage. Try a longer period to view older
            records.
          </p>
        </div>
      )}
      <div className="token-chart-footer">
        <p>
          UTC dates · Partial local history. Claude Code supplies allowance
          windows only.
        </p>
        <button className="button" onClick={exportCsv}>
          <Download size={16} /> Export CSV
        </button>
      </div>
    </section>
  );
}
