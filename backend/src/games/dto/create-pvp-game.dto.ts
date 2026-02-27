import { IsOptional, IsString, IsUUID, MaxLength, MinLength } from 'class-validator';

export class CreatePvpGameDto {
  @IsUUID()
  playerId!: string;

  @IsString()
  @MinLength(3)
  @MaxLength(40)
  name!: string;

  @IsOptional()
  @IsString()
  @MinLength(3)
  @MaxLength(32)
  password?: string;
}
