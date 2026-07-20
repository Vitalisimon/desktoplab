declare module "lucide-react/dist/esm/icons/*.js" {
  import type { ComponentType, SVGProps } from "react";

  type IconProps = SVGProps<SVGSVGElement> & {
    size?: number | string;
    strokeWidth?: number | string;
    absoluteStrokeWidth?: boolean;
  };

  const Icon: ComponentType<IconProps>;
  export default Icon;
}
