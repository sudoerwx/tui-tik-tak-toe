import { IsOptional, IsString, IsUUID, MaxLength, MinLength } from 'class-validator';

export class JoinPvpGameDto {
  @IsUUID()
  playerId!: string;

  @IsOptional()
  @IsString()
  @MinLength(3)
  @MaxLength(32)
  password?: string;
}
