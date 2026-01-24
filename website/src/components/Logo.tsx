interface LogoProps {
  className?: string;
  width?: number;
  height?: number;
}

export function Logo({ className, width = 293, height = 170 }: LogoProps) {
  return (
    <svg
      width={width}
      height={height}
      viewBox="0 0 293 170"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <path
        d="M129.741 0V169.208H94.2925V123.884H35.4487V169.208H0V0H129.741ZM36.0984 82.6616H93.6427V39.9279H36.0984V82.6616Z"
        className="fill-foreground"
      />
      <path
        d="M189.614 0V34.9087H151.74V157.203H274.015M242.866 9.15527e-05V34.9087H280.74V157.203"
        className="stroke-foreground"
        strokeWidth="24"
      />
      <rect x="228.58" y="111.149" width="20" height="20" fill="#B7400F" />
      <rect x="183.9" y="95.4575" width="20" height="20" fill="#B7400F" />
      <rect x="217.41" y="63.7695" width="20" height="20" fill="#B7400F" />
    </svg>
  );
}
