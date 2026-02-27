import { Body, Controller, Get, Param, Post } from '@nestjs/common';

import { CreatePvpGameDto } from './dto/create-pvp-game.dto';
import { CreateSoloGameDto } from './dto/create-solo-game.dto';
import { JoinPvpGameDto } from './dto/join-pvp-game.dto';
import { PlayMoveDto } from './dto/play-move.dto';
import { GamesService } from './games.service';

@Controller('games')
export class GamesController {
  constructor(private readonly gamesService: GamesService) {}

  @Post('solo')
  createSoloGame(@Body() body: CreateSoloGameDto) {
    return this.gamesService.createSoloGame(body);
  }

  @Post('pvp')
  createPvpGame(@Body() body: CreatePvpGameDto) {
    return this.gamesService.createPvpGame(body);
  }

  @Get('pvp/open')
  listOpenPvpGames() {
    return this.gamesService.listOpenPvpGames();
  }

  @Post('pvp/:gameId/join')
  joinPvpGame(@Param('gameId') gameId: string, @Body() body: JoinPvpGameDto) {
    return this.gamesService.joinPvpGame(gameId, body);
  }

  @Get(':gameId')
  getGame(@Param('gameId') gameId: string) {
    return this.gamesService.getGame(gameId);
  }

  @Post(':gameId/move')
  playMove(@Param('gameId') gameId: string, @Body() body: PlayMoveDto) {
    return this.gamesService.playMove(gameId, body);
  }
}
