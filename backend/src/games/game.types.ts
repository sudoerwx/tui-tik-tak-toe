export type PlayerSymbol = 'X' | 'O';
export type GameMode = 'SOLO' | 'PVP';
export type GameStatus = 'WAITING_FOR_PLAYER' | 'IN_PROGRESS' | 'WON' | 'DRAW';

export type GameState = {
  id: string;
  mode: GameMode;
  name: string | null;
  hostPlayerId: string;
  guestPlayerId: string | null;
  board: (PlayerSymbol | null)[];
  currentTurn: PlayerSymbol;
  status: GameStatus;
  winner: PlayerSymbol | null;
  createdAt: string;
  updatedAt: string;
  hasPassword: boolean;
  password: string | null;
};

export type PublicGameState = Omit<GameState, 'password'>;

export const WINNING_LINES: readonly number[][] = [
  [0, 1, 2],
  [3, 4, 5],
  [6, 7, 8],
  [0, 3, 6],
  [1, 4, 7],
  [2, 5, 8],
  [0, 4, 8],
  [2, 4, 6]
] as const;
