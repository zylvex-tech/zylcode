import React from "react";

interface VerificationRungBadgeProps {
  rung: number; // 0-4 representing Rung0-Rung4
  className?: string;
}

export const VerificationRungBadge: React.FC<VerificationRungBadgeProps> = ({
  rung,
  className = "",
}) => {
  const getRungInfo = (rung: number) => {
    switch (rung) {
      case 0:
        return {
          label: "None",
          description: "No verification",
          colorClass: "bg-gray-100 text-gray-800 border-gray-200",
        };
      case 1:
        return {
          label: "Lint/Type-check",
          description: "Basic static analysis",
          colorClass: "bg-blue-100 text-blue-800 border-blue-200",
        };
      case 2:
        return {
          label: "Property Tests",
          description: "Property-based testing",
          colorClass: "bg-green-100 text-green-800 border-green-200",
        };
      case 3:
        return {
          label: "Formal Spec",
          description: "Formal specification",
          colorClass: "bg-purple-100 text-purple-800 border-purple-200",
        };
      case 4:
        return {
          label: "Full Formal",
          description: "Full formal verification",
          colorClass: "bg-amber-100 text-amber-800 border-amber-200",
        };
      default:
        return {
          label: "Unknown",
          description: "Unknown verification level",
          colorClass: "bg-gray-100 text-gray-800 border-gray-200",
        };
    }
  };

  const rungInfo = getRungInfo(rung);

  return (
    <div
      className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium border ${rungInfo.colorClass} ${className}`}
      title={rungInfo.description}
    >
      <span className="mr-1">🔒</span>
      <span className="font-medium">Verification: {rungInfo.label}</span>
      <span className="ml-1.5 text-[0.65rem] opacity-70">({rungInfo.description})</span>
    </div>
  );
};

export default VerificationRungBadge;