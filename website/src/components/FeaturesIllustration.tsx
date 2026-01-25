import { cn } from "@/lib/utils";

interface FeaturesIllustrationProps {
  activeFeature?: number;
  className?: string;
}

export function FeaturesIllustration({
  activeFeature = 0,
  className,
}: FeaturesIllustrationProps) {
  return (
    <svg
      viewBox="0 0 1675 969"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={cn("w-full h-auto", className)}
    >
      <rect width="831.752" height="134.598" transform="matrix(0.866025 0.5 0 1 116.699 416.876)" fill="var(--secondary)" stroke="var(--border)" strokeWidth="2"/>
      {/* <path d="M116.699 416.876L837.017 832.752V967.35L116.699 551.474V416.876Z" fill="black"/> */}
      <path d="M116.699 551.474L115.833 550.974V551.974L116.699 552.474V551.474ZM837.017 967.35V966.35L116.699 550.474V551.474V552.474L837.017 968.35V967.35ZM116.699 551.474L117.565 551.974V417.376L116.699 416.876L115.833 416.376V550.974L116.699 551.474Z" fill="var(--foreground)"/>
      <rect width="831.752" height="134.592" transform="matrix(0.866025 -0.5 0 1 837.027 832.751)" fill="var(--secondary)" stroke="var(--border)" strokeWidth="2"/>
      <path d="M837.027 1L1197.19 208.938L1557.35 416.876L1197.19 624.814L837.027 832.752L476.868 624.814L116.709 416.876L476.868 208.938L837.027 1Z" fill="var(--background)"/>
      <path d="M476.868 208.938L116.709 416.876L476.868 624.814M476.868 208.938L837.027 1L1197.19 208.938M476.868 208.938L1197.19 624.814M1197.19 624.814L1557.35 416.876L1197.19 208.938M1197.19 624.814L837.027 832.752L476.868 624.814M1197.19 624.814L1197.19 759.683M1197.19 208.938L476.868 624.814M476.868 624.814L476.868 759.579" stroke="var(--border)" strokeWidth="2"/>
      <path d="M837.027 1L837.893 0.5L837.027 0L836.161 0.5L837.027 1ZM837.027 1L836.161 1.5L1556.48 417.376L1557.35 416.876L1558.21 416.376L837.893 0.5L837.027 1ZM116.709 416.876L117.575 417.376L837.893 1.5L837.027 1L836.161 0.5L115.843 416.376L116.709 416.876Z" fill="var(--foreground)"/>
      <path d="M837.027 832.751L1557.35 416.876V551.468L837.027 967.344V832.751Z" fill="var(--secondary)"/>
      <path d="M1557.35 551.468V552.468L1558.21 551.968V550.968L1557.35 551.468ZM1557.35 416.876L1556.48 417.376V551.968L1557.35 551.468L1558.21 550.968V416.376L1557.35 416.876ZM1557.35 551.468V550.468L837.027 966.344V967.344V968.344L1557.35 552.468V551.468Z" fill="var(--foreground)"/>

      {/* Panel 0: Bottom-left (Streaming First) */}
      <g className={cn("transition-opacity duration-500", activeFeature === 0 ? "opacity-100" : "opacity-30")}>
        <rect x="0.866025" width="412.771" height="412.771" transform="matrix(0.866025 -0.5 0.866025 0.5 478.776 625.26)" />
        <g opacity="0.1" clipPath="url(#clip0_306_1454)">
          <path d="M913.343 580.749L894.41 591.68C891.066 593.602 888.905 596.104 888.257 598.802C887.609 601.501 888.51 604.248 890.822 606.623L936.705 653.83C936.986 654.125 937.093 654.466 937.01 654.8C936.928 655.133 936.66 655.443 936.247 655.681C935.834 655.92 935.298 656.074 934.72 656.122C934.142 656.17 933.552 656.108 933.04 655.946L740.957 593.708C740.444 593.546 739.855 593.484 739.277 593.531C738.699 593.579 738.163 593.734 737.75 593.972C737.337 594.211 737.069 594.52 736.987 594.854C736.904 595.188 737.011 595.528 737.292 595.823L783.175 643.031C785.478 645.396 786.381 648.131 785.747 650.82C785.113 653.509 782.977 656.005 779.663 657.929L760.654 668.904" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
        </g>
        <g clipPath="url(#clip1_306_1454)" className={cn("transition-all duration-500", activeFeature === 0 ? "stroke-primary -translate-y-12" : "stroke-muted-foreground")}>
          <path d="M913.343 580.749L894.41 591.68C891.066 593.602 888.905 596.104 888.257 598.802C887.609 601.501 888.51 604.248 890.822 606.623L936.705 653.83C936.986 654.125 937.093 654.466 937.01 654.8C936.928 655.133 936.66 655.443 936.247 655.681C935.834 655.92 935.298 656.074 934.72 656.122C934.142 656.17 933.552 656.108 933.04 655.946L740.957 593.708C740.444 593.546 739.855 593.484 739.277 593.531C738.699 593.579 738.163 593.734 737.75 593.972C737.337 594.211 737.069 594.52 736.987 594.854C736.904 595.188 737.011 595.528 737.292 595.823L783.175 643.031C785.478 645.396 786.381 648.131 785.747 650.82C785.113 653.509 782.977 656.005 779.663 657.929L760.654 668.904" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="bevel"/>
        </g>
      </g>

      {/* Panel 1: Bottom-right (Type Safe) */}
      <g className={cn("transition-opacity duration-500", activeFeature === 1 ? "opacity-100" : "opacity-30")}>
        <rect x="0.866025" width="412.771" height="412.771" transform="matrix(0.866025 -0.5 0.866025 0.5 838.934 417.309)" />
        <g opacity="0.1" clipPath="url(#clip2_306_1454)">
          <path d="M1265.86 386.023C1304.03 408.062 1296.4 434.509 1275.71 459.236C1274.61 460.522 1272.78 461.549 1270.52 462.145C1227.69 474.179 1181.88 478.586 1143.71 456.548L1090.27 425.693C1088.24 424.524 1087.11 422.939 1087.11 421.285C1087.11 419.632 1088.24 418.047 1090.27 416.878C1105.54 408.062 1115.46 391.753 1117.14 377.384C1117.38 375.652 1118.68 374.026 1120.8 372.798C1122.93 371.57 1125.75 370.82 1128.75 370.684C1153.79 369.714 1181.88 363.984 1197.15 355.169C1199.18 354 1201.92 353.343 1204.79 353.343C1207.65 353.343 1210.4 354 1212.42 355.169L1265.86 386.023Z" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M1174.25 430.1H1204.79V394.838" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
        </g>
        <g clipPath="url(#clip3_306_1454)" className={cn("transition-all duration-500", activeFeature === 1 ? "stroke-primary -translate-y-12" : "stroke-muted-foreground")}>
          <path d="M1265.86 386.023C1304.03 408.062 1296.4 434.509 1275.71 459.236C1274.61 460.522 1272.78 461.549 1270.52 462.145C1227.69 474.179 1181.88 478.586 1143.71 456.548L1090.27 425.693C1088.24 424.524 1087.11 422.939 1087.11 421.285C1087.11 419.632 1088.24 418.047 1090.27 416.878C1105.54 408.062 1115.46 391.753 1117.14 377.384C1117.38 375.652 1118.68 374.026 1120.8 372.798C1122.93 371.57 1125.75 370.82 1128.75 370.684C1153.79 369.714 1181.88 363.984 1197.15 355.169C1199.18 354 1201.92 353.343 1204.79 353.343C1207.65 353.343 1210.4 354 1212.42 355.169L1265.86 386.023Z" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="bevel"/>
          <path d="M1174.25 430.1H1204.79V394.838" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
        </g>
      </g>

      {/* Panel 2: Top-right (Tool Calling) */}
      <g className={cn("transition-opacity duration-500", activeFeature === 3 ? "opacity-100" : "opacity-30")}>
        <rect x="0.866025" width="412.771" height="412.771" transform="matrix(0.866025 -0.5 0.866025 0.5 478.815 209.344)" />
        <g opacity="0.1" clipPath="url(#clip4_306_1454)">
          <path d="M951.876 221.947C964.526 214.644 964.526 202.803 951.876 195.5C939.227 188.197 918.719 188.197 906.069 195.5C893.42 202.803 893.42 214.644 906.069 221.947C918.719 229.25 939.227 229.25 951.876 221.947Z" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M768.648 221.947C781.297 214.644 781.297 202.804 768.648 195.501C755.999 188.198 735.49 188.198 722.841 195.501C710.192 202.804 710.192 214.644 722.841 221.947C735.49 229.25 755.999 229.25 768.648 221.947Z" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M799.186 177.87L822.089 164.647C826.139 162.309 831.631 160.995 837.358 160.995C843.085 160.995 848.577 162.309 852.627 164.647L906.068 195.501" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M875.533 239.576L852.63 252.8C848.58 255.138 843.088 256.451 837.361 256.451C831.634 256.451 826.141 255.138 822.092 252.8L768.65 221.945" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
        </g>
        <g clipPath="url(#clip5_306_1454)" className={cn("transition-all duration-500", activeFeature === 3 ? "stroke-primary -translate-y-12" : "stroke-muted-foreground")}>
          <path d="M951.876 221.947C964.526 214.644 964.526 202.803 951.876 195.5C939.227 188.197 918.719 188.197 906.069 195.5C893.42 202.803 893.42 214.644 906.069 221.947C918.719 229.25 939.227 229.25 951.876 221.947Z" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M768.648 221.947C781.297 214.644 781.297 202.804 768.648 195.501C755.999 188.198 735.49 188.198 722.841 195.501C710.192 202.804 710.192 214.644 722.841 221.947C735.49 229.25 755.999 229.25 768.648 221.947Z" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M799.186 177.87L822.089 164.647C826.139 162.309 831.631 160.995 837.358 160.995C843.085 160.995 848.577 162.309 852.627 164.647L906.068 195.501" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M875.533 239.576L852.63 252.8C848.58 255.138 843.088 256.451 837.361 256.451C831.634 256.451 826.141 255.138 822.092 252.8L768.65 221.945" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
        </g>
      </g>

      {/* Panel 3: Top-left (Prompt Caching) */}
      <g className={cn("transition-opacity duration-500", activeFeature === 2 ? "opacity-100" : "opacity-30")}>
        <rect x="0.866025" width="412.771" height="412.771" transform="matrix(0.866025 -0.5 0.866025 0.5 118.61 417.309)" />
        <g opacity="0.1" clipPath="url(#clip6_306_1454)">
          <path d="M461.451 346.415L354.568 408.124C346.135 412.993 346.135 420.886 354.568 425.755L461.451 487.464C469.884 492.333 483.556 492.333 491.989 487.464L598.872 425.755C607.304 420.886 607.304 412.993 598.872 408.124L491.989 346.415C483.556 341.546 469.884 341.546 461.451 346.415Z" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M430.918 408.122H491.994V443.384" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
        </g>
        <g clipPath="url(#clip7_306_1454)" className={cn("transition-all duration-500", activeFeature === 2 ? "stroke-primary -translate-y-12" : "stroke-muted-foreground")}>
          <path d="M461.451 346.415L354.568 408.124C346.135 412.993 346.135 420.886 354.568 425.755L461.451 487.464C469.884 492.333 483.556 492.333 491.989 487.464L598.872 425.755C607.304 420.886 607.304 412.993 598.872 408.124L491.989 346.415C483.556 341.546 469.884 341.546 461.451 346.415Z" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
          <path d="M430.918 408.122H491.994V443.384" stroke="white" strokeWidth="17.6311" strokeLinecap="round" strokeLinejoin="round"/>
        </g>
      </g>

      {/* <path d="M839.71 319.158L990.558 406.25L839.71 493.341L688.863 406.25L839.71 319.158Z" fill="black" stroke="#525252" strokeWidth="2"/> */}
      {/* <path d="M991.065 407.957V422.247L840.935 508.924V494.634L991.065 407.957ZM838.486 494.634V508.924L688.356 422.247V407.957L838.486 494.634Z" fill="#4C4C4C" stroke="white" strokeWidth="2"/> */}
      {/* <rect width="124.334" height="72.0489" transform="matrix(0.866025 -0.5 0.866025 0.5 754.674 419.321)" fill="black"/> */}
      {/* <path d="M802.516 391.699L864.913 427.723L851.841 435.27L835.127 425.621L813.429 438.149L830.142 447.798L817.07 455.345L754.674 419.321L802.516 391.699ZM798.467 429.234L819.687 416.983L803.929 407.885L782.709 420.136L798.467 429.234Z" fill="white"/> */}
      {/* <path d="M824.595 378.952L837.468 386.384L823.502 394.447L868.598 420.484L913.688 394.451M844.232 367.614L857.105 375.046L871.071 366.983L916.167 393.02" stroke="white" strokeWidth="10.2192"/> */}
      {/* <rect width="8.51601" height="8.51601" transform="matrix(0.866025 -0.5 0.866025 0.5 879.949 394.32)" fill="#B7400F"/> */}
      {/* <rect width="8.51601" height="8.51601" transform="matrix(0.866025 -0.5 0.866025 0.5 857.688 400.492)" fill="#B7400F"/> */}
      {/* <rect width="8.51601" height="8.51601" transform="matrix(0.866025 -0.5 0.866025 0.5 858.359 386.611)" fill="#B7400F"/> */}
      <defs>
        <clipPath id="clip0_306_1454">
          <rect width="211.573" height="211.573" fill="var(--foreground)" transform="matrix(0.866025 -0.5 0.866025 0.5 653.77 624.827)"/>
        </clipPath>
        <clipPath id="clip1_306_1454">
          <rect width="211.573" height="211.573" fill="var(--foreground)" transform="matrix(0.866025 -0.5 0.866025 0.5 653.77 624.827)"/>
        </clipPath>
        <clipPath id="clip2_306_1454">
          <rect width="211.573" height="211.573" fill="var(--foreground)" transform="matrix(0.866025 -0.5 0.866025 0.5 1013.93 416.876)"/>
        </clipPath>
        <clipPath id="clip3_306_1454">
          <rect width="211.573" height="211.573" fill="var(--foreground)" transform="matrix(0.866025 -0.5 0.866025 0.5 1013.93 416.876)"/>
        </clipPath>
        <clipPath id="clip4_306_1454">
          <rect width="211.573" height="211.573" fill="var(--foreground)" transform="matrix(0.866025 -0.5 0.866025 0.5 654.133 208.723)"/>
        </clipPath>
        <clipPath id="clip5_306_1454">
          <rect width="211.573" height="211.573" fill="var(--foreground)" transform="matrix(0.866025 -0.5 0.866025 0.5 654.133 208.723)"/>
        </clipPath>
        <clipPath id="clip6_306_1454">
          <rect width="211.573" height="211.573" fill="var(--foreground)" transform="matrix(0.866025 -0.5 0.866025 0.5 293.494 416.938)"/>
        </clipPath>
        <clipPath id="clip7_306_1454">
          <rect width="211.573" height="211.573" fill="var(--foreground)" transform="matrix(0.866025 -0.5 0.866025 0.5 293.494 416.938)"/>
        </clipPath>
      </defs>
    </svg>
  );
}
