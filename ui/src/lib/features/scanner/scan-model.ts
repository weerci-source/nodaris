export type Band = "24g" | "5g" | "6g" | "unknown";
export type ScanSort = "quality" | "ssid" | "frequency";

export type AccessPoint = {
    ssid: string;
    bssid: string;
    rssiDbm: number | null;
    qualityPercent: number | null;
    frequencyMhz: number;
    channel: number | null;
    channelWidthMhz: number | null;
    isConnected: boolean;
};

export function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}

function integer(value: unknown, min: number, max = Number.MAX_SAFE_INTEGER): value is number {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= min && value <= max;
}

function measuredRssi(value: unknown): number | null {
    if (!isRecord(value) || !isRecord(value.state) || value.state.status !== "available") return null;
    const dbm = value.state.value;
    return integer(dbm, -2147483648, 2147483647) ? dbm : null;
}

export function decodeScan(value: unknown): AccessPoint[] | null {
    if (!Array.isArray(value)) return null;
    const points: AccessPoint[] = [];

    for (const item of value) {
        if (
            !isRecord(item) ||
            typeof item.ssid !== "string" ||
            typeof item.bssid !== "string" ||
            !integer(item.frequency_mhz, 1) ||
            typeof item.is_connected !== "boolean" ||
            !(item.signal_quality_percent === null || integer(item.signal_quality_percent, 0, 100))
        ) return null;

        points.push({
            ssid: item.ssid,
            bssid: item.bssid,
            rssiDbm: measuredRssi(item.rssi_dbm),
            qualityPercent: item.signal_quality_percent,
            frequencyMhz: item.frequency_mhz,
            channel: integer(item.channel, 1) ? item.channel : null,
            channelWidthMhz: integer(item.channel_width_mhz, 1) ? item.channel_width_mhz : null,
            isConnected: item.is_connected,
        });
    }

    return points;
}

export function bandOf(mhz: number): Band {
    if (mhz >= 2400 && mhz <= 2500) return "24g";
    if (mhz >= 4900 && mhz < 5925) return "5g";
    if (mhz >= 5925 && mhz <= 7125) return "6g";
    return "unknown";
}

export function selectPoints(
    points: readonly AccessPoint[],
    query: string,
    band: Band | "all",
    sort: ScanSort,
    locale: string,
): AccessPoint[] {
    const needle = query.trim().toLocaleLowerCase(locale);
    const collator = new Intl.Collator(locale, { sensitivity: "base", numeric: true });

    return points.filter((point) =>
        (band === "all" || bandOf(point.frequencyMhz) === band) &&
        (point.ssid.toLocaleLowerCase(locale).includes(needle) || point.bssid.toLowerCase().includes(needle))
    ).sort((a, b) => {
        const order = sort === "ssid"
            ? collator.compare(a.ssid, b.ssid)
            : sort === "frequency"
              ? a.frequencyMhz - b.frequencyMhz
              : (b.qualityPercent ?? -1) - (a.qualityPercent ?? -1);
        return order || collator.compare(a.ssid, b.ssid) || collator.compare(a.bssid, b.bssid);
    });
}
