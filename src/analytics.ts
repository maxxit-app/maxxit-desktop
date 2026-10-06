export interface TokenDay {
  startDate: string;
  tokens: number;
}

// Local session totals use UTC dates. Missing dates have unknown coverage.
export function tokenSeries(
  records: TokenDay[],
  days: number,
  now = new Date(),
) {
  const totals = new Map<string, number>();
  for (const record of records) {
    if (
      !/^\d{4}-\d{2}-\d{2}$/.test(record.startDate) ||
      !Number.isFinite(record.tokens) ||
      record.tokens < 0
    )
      continue;
    totals.set(
      record.startDate,
      (totals.get(record.startDate) ?? 0) + record.tokens,
    );
  }
  const end = Date.UTC(
    now.getUTCFullYear(),
    now.getUTCMonth(),
    now.getUTCDate(),
  );
  return Array.from({ length: days }, (_, index) => {
    const date = new Date(end - (days - index - 1) * 86_400_000)
      .toISOString()
      .slice(0, 10);
    return { startDate: date, tokens: totals.get(date) ?? null };
  });
}
export function tokenSummary(
  series: { startDate: string; tokens: number | null }[],
) {
  const observed = series.filter((day) => day.tokens !== null);
  const total = observed.reduce((sum, day) => sum + (day.tokens ?? 0), 0);
  const peak = observed.reduce<(typeof series)[number] | null>(
    (best, day) =>
      !best || (day.tokens ?? 0) > (best.tokens ?? 0) ? day : best,
    null,
  );
  return {
    total,
    observedDays: observed.length,
    average: observed.length ? total / observed.length : null,
    peak,
  };
}
