import { useProject } from "../stores/project.store";
export function useLanguage() {
  const language = useProject((s) => s.settings.language);
  return (id: string, en: string) => (language === "id" ? id : en);
}
