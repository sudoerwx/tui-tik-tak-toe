import { ValidationPipe } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

import { AppModule } from './app.module';

async function bootstrap() {
  const app = await NestFactory.create(AppModule);

  // ValidationPipe gives us runtime safety for DTOs and strips unknown fields.
  app.useGlobalPipes(
    new ValidationPipe({
      whitelist: true,
      forbidNonWhitelisted: true,
      transform: true
    })
  );

  const port = 3000;
  await app.listen(port);
  // eslint-disable-next-line no-console
  console.log(`Backend started on http://localhost:${port}`);
}

void bootstrap();
