import { useEffect, useState } from "react";
export function ParamField({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (n: number) => void;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  const valid =
    text.trim() !== "" &&
    Number.isFinite(Number(text)) &&
    Number(text) >= min &&
    Number(text) <= max &&
    (step !== 1 || Number.isInteger(Number(text)));
  return (
    <div className="parameter">
      <div>
        <label>
          {label}
          <input
            aria-label={label}
            type="number"
            value={text}
            min={min}
            max={max}
            step={step}
            aria-invalid={!valid}
            onChange={(e) => {
              setText(e.target.value);
              const n = Number(e.target.value);
              if (
                e.target.value !== "" &&
                Number.isFinite(n) &&
                n >= min &&
                n <= max &&
                (step !== 1 || Number.isInteger(n))
              )
                onChange(n);
            }}
            onBlur={() => setText(String(value))}
          />
        </label>
      </div>
      <input
        aria-label={`${label} slider`}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      {!valid && (
        <small role="alert">
          {min} – {max}
        </small>
      )}
    </div>
  );
}
