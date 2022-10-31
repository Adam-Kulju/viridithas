
import chess
import chess.pgn
import tqdm

# read command line args
import sys
if len(sys.argv) != 3:
    print("Usage: texelconversion.py <INPUTPGN> <OUTPUTTXT>")
    exit(1)
PGN = sys.argv[1]
OUT = sys.argv[2]

def main():
    counter = 0
    wins, draws, losses = 0, 0, 0
    pbar = tqdm.tqdm()
    with open(PGN, "r") as pgn:
        with open(OUT, "a") as texel_data:
            while True:
                if counter & 0xFF == 0:
                    pbar.update(0xFF)
                game = chess.pgn.read_game(pgn)
                if game is None: # EOF
                    break
                result = game.headers["Result"]
                if result == "1-0":
                    result = 1.0
                    wins += 1
                elif result == "0-1":
                    result = 0.0
                    losses += 1
                elif result == "1/2-1/2":
                    result = 0.5
                    draws += 1
                else:
                    print(f"Unknown result: {result}")
                    exit(1)
                nodes = game.mainline()
                for node in nodes:
                    evaluation = node.comment
                    if "book" in evaluation or "M" in evaluation:
                        continue
                    board = node.board()
                    texel_data.write(f"{board.fen()};{result}\n")
                counter += 1
    pbar.close()
    
    print(f"Wins:   {wins}")
    print(f"Draws:  {draws}")
    print(f"Losses: {losses}")

main()