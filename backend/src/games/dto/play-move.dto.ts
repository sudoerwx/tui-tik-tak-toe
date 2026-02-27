import { IsInt, IsUUID, Max, Min } from 'class-validator';

export class PlayMoveDto {
  @IsUUID()
  playerId!: string;

  @IsInt()
  @Min(0)
  @Max(8)
  index!: number;
}
