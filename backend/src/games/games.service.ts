import {
  BadRequestException,
  Injectable,
  NotFoundException,
  UnauthorizedException
} from '@nestjs/common';
import { randomUUID } from 'crypto';

import { CreatePvpGameDto } from './dto/create-pvp-game.dto';
import { CreateSoloGameDto } from './dto/create-solo-game.dto';
import { JoinPvpGameDto } from './dto/join-pvp-game.dto';
import { PlayMoveDto } from './dto/play-move.dto';
import { GameState, PlayerSymbol, PublicGameState, WINNING_LINES } from './game.types';

@Injectable()
export class GamesService {
  // In-memory storage is enough for this local-learning project.
  // If you move to production, replace this with a repository/database layer.
  private readonly games = new Map<string, GameState>();

  /**
   * Creates a new solo game where the player competes against an AI.
   *
   * @param dto - The DTO containing player and game setup information.
   * @returns Public representation of the newly created game state.
   */
  createSoloGame(dto: CreateSoloGameDto): PublicGameState {
    const now = new Date().toISOString();
    const game: GameState = {
      id: randomUUID(),
      mode: 'SOLO',
      name: `Solo game (${dto.clientName})`,
      hostPlayerId: dto.playerId,
      guestPlayerId: 'AI',
      board: Array.from({ length: 9 }, () => null),
      currentTurn: 'X',
      status: 'IN_PROGRESS',
      winner: null,
      createdAt: now,
      updatedAt: now,
      hasPassword: false,
      password: null
    };

    this.games.set(game.id, game);
    return this.toPublic(game);
  }

  /**
   * Creates a new player-versus-player game.
   * PVP games are initialized with the creator as the host player, awaiting the guest player.
   * @param dto - Contains game name, creator player ID, and an optional password.
   * @returns Public representation of the game state.
   */
  createPvpGame(dto: CreatePvpGameDto): PublicGameState {
    const now = new Date().toISOString();
    const game: GameState = {
      id: randomUUID(),
      mode: 'PVP',
      name: dto.name,
      hostPlayerId: dto.playerId,
      guestPlayerId: null,
      board: Array.from({ length: 9 }, () => null),
      currentTurn: 'X',
      status: 'WAITING_FOR_PLAYER',
      winner: null,
      createdAt: now,
      updatedAt: now,
      hasPassword: Boolean(dto.password),
      password: dto.password ?? null
    };

    this.games.set(game.id, game);
    return this.toPublic(game);
  }

  /**
   * Lists all open PvP games.
   * These are the games that are in the WAITING_FOR_PLAYER state,
   * meaning they have a host but no guest player yet.
   *
   * Games are returned in descending order of creation time.
   * @returns Array of public game states.
   */
  listOpenPvpGames(): PublicGameState[] {
    const openGames = [...this.games.values()]
      .filter((game) => game.mode === 'PVP' && game.status === 'WAITING_FOR_PLAYER')
      .sort((a, b) => b.createdAt.localeCompare(a.createdAt));

    return openGames.map((game) => this.toPublic(game));
  }

  /**
   * Lets a player join an existing PvP game.
   *
   * Validates the game mode, player roles, and passwords, if applicable.
   * Updates the game status to 'IN_PROGRESS' once the guest player joins.
   *
   * @param gameId - The ID of the game to join.
   * @param dto - The DTO containing player ID and optional password.
   * @throws BadRequestException | UnauthorizedException
   * @returns Updated public game state.
   */
  joinPvpGame(gameId: string, dto: JoinPvpGameDto): PublicGameState {
    const game = this.getExistingGame(gameId);

    if (game.mode !== 'PVP') {
      throw new BadRequestException('Only PvP games can be joined');
    }

    if (game.status !== 'WAITING_FOR_PLAYER') {
      throw new BadRequestException('This game is not waiting for a second player');
    }

    if (game.hostPlayerId === dto.playerId) {
      throw new BadRequestException('Host cannot join the same game as guest');
    }

    if (game.password && game.password !== dto.password) {
      throw new UnauthorizedException('Invalid game password');
    }

    game.guestPlayerId = dto.playerId;
    game.status = 'IN_PROGRESS';
    game.updatedAt = new Date().toISOString();

    return this.toPublic(game);
  }

  /**
   * Retrieves public game details based on the game ID provided.
   *
   * @param gameId - The unique identifier for the game to retrieve.
   * @returns Public representation of the game state.
   */
  getGame(gameId: string): PublicGameState {
    return this.toPublic(this.getExistingGame(gameId));
  }

  /**
   * Processes a player's move within a game.
   * Validates the game status, intended board position, and player turn.
   * Automatically handles AI moves in solo mode.
   *
   * @param gameId - The ID of the game where the move occurs.
   * @param dto - DTO containing the player's ID and the board index for the move.
   * @throws BadRequestException | UnauthorizedException
   * @returns The updated public game state after the move.
   */
  playMove(gameId: string, dto: PlayMoveDto): PublicGameState {
    const game = this.getExistingGame(gameId);

    if (!['IN_PROGRESS'].includes(game.status)) {
      throw new BadRequestException('Game is not active');
    }

    if (game.board[dto.index] !== null) {
      throw new BadRequestException('Cell is already occupied');
    }

    // Turn ownership convention:
    // X is always host, O is always guest/AI.
    const expectedPlayerId = game.currentTurn === 'X' ? game.hostPlayerId : game.guestPlayerId;

    if (expectedPlayerId !== dto.playerId) {
      throw new UnauthorizedException('It is not your turn');
    }

    // Apply move optimistically, then compute winner/draw/next turn.
    game.board[dto.index] = game.currentTurn;
    this.applyPostMoveState(game);

    // In solo mode, AI plays immediately after the human turn.
    if (game.mode === 'SOLO' && game.status === 'IN_PROGRESS' && game.currentTurn === 'O') {
      const aiMove = this.selectAiMove(game.board);
      game.board[aiMove] = 'O';
      this.applyPostMoveState(game);
    }

    game.updatedAt = new Date().toISOString();
    return this.toPublic(game);
  }

  /**
   * Updates the game state following a player's move.
   * Checks for winning conditions, determines if the game is a draw, or sets the next player's turn.
   *
   * @param game - The current game state to be updated.
   */
  private applyPostMoveState(game: GameState): void {
    // This function is pure game-rules logic and does not care about mode.
    // Keeping this split makes it easy to test or reuse in another transport layer.
    const winner = this.getWinner(game.board);
    if (winner) {
      game.status = 'WON';
      game.winner = winner;
      return;
    }

    const hasEmptyCell = game.board.some((cell) => cell === null);
    if (!hasEmptyCell) {
      game.status = 'DRAW';
      return;
    }

    game.currentTurn = game.currentTurn === 'X' ? 'O' : 'X';
  }

  /**
   * Retrieves an existing game from the in-memory store by its ID.
   * Throws an exception if the game is not found.
   *
   * @param gameId - The unique ID of the game.
   * @returns The complete game state.
   * @throws NotFoundException if the game is missing.
   */
  private getExistingGame(gameId: string): GameState {
    const game = this.games.get(gameId);
    if (!game) {
      throw new NotFoundException('Game was not found');
    }

    return game;
  }

  private toPublic(game: GameState): PublicGameState {
    // Never expose password in API responses.
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const { password: _password, ...publicGame } = game;
    return publicGame;
  }

  private getWinner(board: (PlayerSymbol | null)[]): PlayerSymbol | null {
    for (const [a, b, c] of WINNING_LINES) {
      if (board[a] && board[a] === board[b] && board[b] === board[c]) {
        return board[a];
      }
    }

    return null;
  }

  private selectAiMove(board: (PlayerSymbol | null)[]): number {
    // "Simple but decent" AI strategy:
    // 1) win now, 2) block opponent win, 3) center, 4) corners, 5) first free.
    const canWinAsO = this.findFinishingMove(board, 'O');
    if (canWinAsO !== null) {
      return canWinAsO;
    }

    const blockX = this.findFinishingMove(board, 'X');
    if (blockX !== null) {
      return blockX;
    }

    if (board[4] === null) {
      return 4;
    }

    const corners = [0, 2, 6, 8];
    for (const corner of corners) {
      if (board[corner] === null) {
        return corner;
      }
    }

    const fallback = board.findIndex((cell) => cell === null);
    if (fallback === -1) {
      throw new BadRequestException('No legal AI move found');
    }

    return fallback;
  }

  /**
   * Determines if a winning or blocking move is possible for the given symbol.
   *
   * @param board - Current state of the game board.
   * @param symbol - Player's symbol ('X' or 'O') to check for potential finishing moves.
   * @returns The index of the finishing move, or null if no such move exists.
   */
  private findFinishingMove(board: (PlayerSymbol | null)[], symbol: PlayerSymbol): number | null {
    for (const [a, b, c] of WINNING_LINES) {
      const line = [board[a], board[b], board[c]];
      const symbolCount = line.filter((value) => value === symbol).length;
      const emptyCount = line.filter((value) => value === null).length;

      if (symbolCount === 2 && emptyCount === 1) {
        if (board[a] === null) {
          return a;
        }

        if (board[b] === null) {
          return b;
        }

        return c;
      }
    }

    return null;
  }
}
