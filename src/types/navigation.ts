export const appPages = [
  "today",
  "chat",
  "goals",
  "actions",
  "history",
  "insights",
  "settings",
] as const;

export type AppPage = (typeof appPages)[number];

export const pageLabels: Record<AppPage, string> = {
  today: "Today",
  goals: "Goals",
  actions: "Actions",
  history: "History",
  insights: "Insights",
  chat: "Chat",
  settings: "Settings",
};
