export interface MonitorCaptureDto {
  index: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  path: string;
}

export interface RectDto {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface CropResultDto {
  path: string;
  width: number;
  height: number;
}

export interface StagedImageDto {
  path: string;
  width: number;
  height: number;
}

export interface ChatMessageDto {
  id: string;
  role: "user" | "assistant" | string;
  text: string;
  images: string[];
  createdAt: number;
}

export interface ConversationViewDto {
  id: string;
  messages: ChatMessageDto[];
  stagedImage: StagedImageDto | null;
}

export interface SessionChatDto {
  crop: CropResultDto;
  conversation: ConversationViewDto;
}

export interface PublicConfigDto {
  baseUrl: string;
  model: string;
  hasApiKey: boolean;
  theme: "system" | "light" | "dark";
  quickPrompts: string[];
}

export interface BubbleTextPayload {
  text: string;
  side: "left" | "right";
}
