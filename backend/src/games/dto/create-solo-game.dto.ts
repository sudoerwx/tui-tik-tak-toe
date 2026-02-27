import { IsString, IsUUID } from 'class-validator';

export class CreateSoloGameDto {
  @IsUUID()
  playerId!: string;

  @IsString()
  clientName!: string;
}
