use std::collections::HashSet;
use std::collections::VecDeque;

struct Cell {
    potentials: HashSet<u32>,
}

impl Cell{
    fn new (value: u32) -> Cell {
        let mut cell = Cell {
            potentials: HashSet::new(),
        };
        if value == 0 {
            for number in 1..10 {
                cell.potentials.insert(number);
            }
        } else {
            cell.potentials.insert(value);
        }
        return cell;
    }
    fn resolved_to( &self) -> u32 {
        if self.potentials.len() == 1 {
            return *self.potentials.iter().next().unwrap();
        } else {
            return 0;
        }
    }
    fn is_solved(&self) -> bool {       
        self.potentials.len() == 1
    }
    fn reduce_potentials(&mut self, value: u32) {
        self.potentials.remove(&value);
    }
}

struct SolvedCell {
    x: u32,
    y: u32,
    value: u32,
}

struct Map {
    cells: Vec<Vec<Cell>>,
    solved_cells: VecDeque<SolvedCell>,
}

impl Map{
    fn new(rows: [[u32; 9]; 9]) -> Map {
        let mut cells: Vec<Vec<Cell>> = Vec::new();
        let mut solved_cells = VecDeque::new();
        for y in 0..rows.len() {
            let mut new_row: Vec<Cell> = Vec::new();
            for x in 0..rows[y].len() {
                let value = rows[y][x];
                let mut cell = Cell::new(value);
                new_row.push(cell);
                if value != 0 {
                    solved_cells.push_back(SolvedCell {
                        x: x as u32,
                        y: y as u32,
                        value: value,
                    });
                }
            }
            cells.push(new_row);
        }
        Map {
            cells: cells,
            solved_cells : solved_cells
        }
    }
    fn print(&self) {
        for y in 0..self.cells.len() {
            for x in 0..self.cells[y].len() {
                let buf = if x % 3 == 0 { " " } else { "" };
                print!( "{}{}", buf, self.cells[y][x].resolved_to() );
            }
            print!("\n");
            if (y + 1) % 3 == 0 {
                print!("\n");
            }
        }
    }

    fn solve(&mut self) {
        loop {
            let solved = self.solved_cells.pop_front();
            if solved.is_none() {
                break;
            }
            self.reduce_potentials(solved.unwrap());
        }
    }

    fn reduce_cell_potentials( &mut self, x : u32, y : u32, value : u32 ) {
        let num_solutions = self.solved_cells.len();
        {
            let cell: &mut Cell = &mut self.cells[y as usize][x as usize];
            if !(cell.is_solved()) {
                cell.reduce_potentials(value);
                if cell.is_solved() {
                    // the last operation solved the cell
                    self.solved_cells.push_back(SolvedCell {
                        x: x,
                        y: y,
                        value: cell.resolved_to(),
                    })
                }
            }
        }
        if num_solutions != self.solved_cells.len() {
            self.print(  );
        }
    }

    fn reduce_potentials(&mut self, solved: SolvedCell) {
        // horizontal
        for x in 0..9 {
            self.reduce_cell_potentials( x, solved.y, solved.value );
        }
        // vertical
        for y in 0..9 {
            self.reduce_cell_potentials( solved.x, y, solved.value );
        }
        // square
        let sq_x: u32 = (solved.x / 3) * 3;
        let sq_y: u32 = (solved.y / 3) * 3;
        for y in 0..3 {
            for x in 0..3 {
                self.reduce_cell_potentials( sq_x+x, sq_y+y, solved.value );
            }
        }
    }
}

fn main() {
    let mut map = Map::new([
        [7, 0, 6, 0, 4, 0, 9, 0, 0],
        [0, 0, 0, 1, 6, 2, 0, 7, 0],
        [5, 0, 3, 0, 0, 0, 1, 0, 4],
        [0, 5, 0, 6, 0, 4, 0, 1, 0],
        [4, 3, 0, 0, 0, 0, 0, 2, 6],
        [0, 6, 0, 3, 0, 9, 0, 4, 0],
        [3, 0, 4, 0, 0, 0, 6, 0, 8],
        [0, 7, 0, 8, 3, 6, 0, 0, 0],
        [0, 0, 1, 0, 9, 0, 2, 0, 7],
    ]);
    map.print();
    map.solve();
}
