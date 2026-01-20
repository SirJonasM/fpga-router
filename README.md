Project structure.
- routing-fpga: The PathFinder Algorithm
- fpga-backend: The backend that calls the PathFinder Algorithm under the hood.
- fpga-frontend: Vite React frontend to interact with Backend and run some tests.

For the exercise only routing-fpga is necessary but the other two are nice to have.

building this project:
- You need cargo (rust toolchain)
- go into routing-fpga and run cargo build --release

Running the project:
- go into routing-fpga and run cargo run --release
This will run the tests for Simple and Steiner Solver that were made in the document.
You can change tests in the get\_test\_cases function
The Code is documented.
A run takes some time and depends mostly on the MAX\_ITERATION in lib.rs as some test cases are not solvable.

Now the Browser version:
I am not sure if this will work on other machines...
To run the frontend you need to have installed npm then you can install all dependencies with npm i and run it with npm run dev.
Then start the backend with `cargo run --release`

You will find the frontend at localhost:5173.

There you can start an arbitrary test.
When you click on the test you will be redirected to an info site. 
When it finished successfully you can also visualieze  the result as a graph.

