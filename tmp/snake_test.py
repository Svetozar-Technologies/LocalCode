import pygame
import time
import random

def draw_snake(snake_block, snake_list):
    for x in snake_list:
        pygame.draw.rect(screen, black, [x[0], x[1], snake_block, snake_block])

def gameLoop():
    game_over = False
    game_close = False
    
x1 = dis_width / 2
y1 = dis_height / 2

x1_change = 0        
y1_change = 0

snake_List = []
Length_of_snake = 1

while not game_over:

    while game_close == True:
        screen.fill(blue)
        message("You Lost! Press Q-Quit or C-Play Again", red)
        pygame.display.update()
 
        for event in pygame.event.get():
            if event.type == pygame.KEYDOWN:
                if event.key == pygame.K_q:
                    game_over = True
                    game_close = False
                if event.key == pygame.K_c:
                    gameLoop()
 
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            game_over = True
        if event.type == pygame.KEYDOWN:
            if event.key == pygame.K_LEFT:
                x1_change = -snake_block_size
                y1_change = 0
            elif event.key == pygame.K_RIGHT:
                x1_change = snake_block_size
                y1_change = 0
            elif event.key == pygame.K_UP:
                y1_change = -snake_block_size
                x1_change = 0
            elif event.key == pygame.K_DOWN:
                y1_change = snake_block_size
                x1_change = 0
 
x1 += x1_change
y1 += y1_change
screen.fill(blue)
pygame.draw.rect(screen, green, [foodx, foody, snake_block_size, snake_block_size])
snake_Head = []
snake_Head.append(x1)
snake_Head.append(y1)
snake_List.append(snake_Head)
if len(snake_List) > Length_of_snake:
    del snake_List[0]

for x in snake_List[:-1]:
    if x == snake_Head:
        game_close = True
 
draw_snake(snake_block_size, snake_List)

pygame.display.update()

clock.tick(snake_speed)

pygame.quit()
quit()

if __name__ == '__main__':
    gameLoop()