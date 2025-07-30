import { useState, type FC } from "react";

interface TokenImageProps {
  src?: string;
  alt: string;
  symbol?: string;
  className?: string;
}

const TokenImage: FC<TokenImageProps> = ({
  src,
  alt,
  symbol = "T",
  className = "w-6 h-6",
}) => {
  const [imageError, setImageError] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  const handleImageError = (): void => {
    setImageError(true);
    setIsLoading(false);
  };

  const handleImageLoad = (): void => {
    setIsLoading(false);
  };

  // Fallback content: first letter of symbol
  const fallbackContent = symbol ? symbol.charAt(0).toUpperCase() : "T";

  if (!src || imageError) {
    return (
      <div
        className={`${className} rounded-full bg-gradient-to-r from-blue-500 to-purple-600 flex items-center justify-center text-white font-semibold text-sm`}
        title={alt}
      >
        {fallbackContent}
      </div>
    );
  }

  return (
    <div className={`${className} relative`}>
      {isLoading && (
        <div className={`${className} rounded-full bg-gray-700 animate-pulse`} />
      )}
      <img
        src={src}
        alt={alt}
        className={`${className} rounded-full ${isLoading ? "opacity-0" : "opacity-100"} transition-opacity duration-200`}
        onError={handleImageError}
        onLoad={handleImageLoad}
        loading="lazy"
      />
    </div>
  );
};

export default TokenImage;
