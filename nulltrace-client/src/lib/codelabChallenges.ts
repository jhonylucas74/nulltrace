/**
 * Codelab challenge definitions.
 * Each challenge has 3 test cases; the user must pass all 3 to complete it.
 *
 * DIFFICULTY GUIDE:
 *   easy   — Tutorial mode: teaches one Luau concept per exercise.
 *            Rich descriptions + commented starter code. For total beginners.
 *   medium — Algorithmic thinking. Roughly “easy” level on typical coding challenge sites.
 *   hard   — Real algorithms required. Roughly “medium” level on typical coding challenge sites.
 */

export type Difficulty = "easy" | "medium" | "hard";

export interface TestCase {
  id: number;
  /** Human-readable description shown in the UI */
  input: string;
  /** Exact expected print output (newline-separated) */
  expectedOutput: string;
  /** Luau code appended after user code to invoke their function */
  setupCode: string;
}

export interface Challenge {
  id: string;
  title: string;
  difficulty: Difficulty;
  /** Problem statement shown on the left panel */
  description: string;
  /** Input/output examples shown below the description */
  examples: string;
  /** Initial code placed in the editor */
  starterCode: string;
  /** Exactly 3 test cases */
  testCases: [TestCase, TestCase, TestCase];
}

export const CHALLENGES: Challenge[] = [
  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //  EASY  (14 tutorial challenges — one concept each)
  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  // ── Tutorial 1 of 14: Functions and print() ───────────────────────────────
  {
    id: "hello-world",
    title: "Hello, World!",
    difficulty: "easy",
    description: `Welcome to Codelab! Let's learn Luau step by step.

── What is a Function? ──────────────────────────
A function is a named block of code you can run whenever you want.
Define it with the 'function' keyword:

  function myFunction()
    -- code goes inside here
  end

Call (run) it by writing its name followed by ():

  myFunction()

── What is print()? ─────────────────────────────
print() is a built-in function that shows text in the console.
Text wrapped in quotes is called a "string":

  print("Hello!")    → shows: Hello!
  print("Luau")      → shows: Luau

── Your Task ────────────────────────────────────
Complete the hello() function so it prints:

  Hello, World!`,
    examples: `-- Call it like this:\nhello()\n\n-- Expected output:\nHello, World!`,
    starterCode: `-- TUTORIAL 1 of 14: Functions and print()
-- ─────────────────────────────────────────────
-- A function is defined with 'function' and ends with 'end'.
-- print() shows text on screen. Text must be inside quotes.
--
-- Example:
--   print("Good morning!")   → Good morning!
--
-- YOUR TASK: Make hello() print: Hello, World!

function hello()
  -- Write your code here:

end`,
    testCases: [
      { id: 1, input: "hello()", expectedOutput: "Hello, World!", setupCode: "hello()" },
      { id: 2, input: "hello() × 2", expectedOutput: "Hello, World!\nHello, World!", setupCode: "hello()\nhello()" },
      { id: 3, input: "hello() × 3", expectedOutput: "Hello, World!\nHello, World!\nHello, World!", setupCode: "hello()\nhello()\nhello()" },
    ],
  },

  // ── Tutorial 2 of 14: Local Variables ────────────────────────────────────
  {
    id: "store-values",
    title: "Storing Values",
    difficulty: "easy",
    description: `── What is a Variable? ──────────────────────────
A variable is a named container that stores a value.
In Luau, always use the 'local' keyword when creating variables:

  local age = 25
  local language = "Luau"
  local score = 100

After declaring, use the variable anywhere in the function:

  print(age)        → 25
  print(language)   → Luau

── Why 'local'? ─────────────────────────────────
'local' keeps the variable inside the current function.
It's good practice — prevents accidental bugs in other code.

── Two Kinds of Values ──────────────────────────
  local count = 42         Numbers → no quotes needed
  local word = "hello"     Strings → must be in quotes

── Your Task ────────────────────────────────────
Fix the variable values in declareVars() so the output is:

  Name: Luau
  Version: 6`,
    examples: `-- Expected output:\nName: Luau\nVersion: 6`,
    starterCode: `-- TUTORIAL 2 of 14: Local Variables
-- ─────────────────────────────────────────────
-- Use 'local' to create variables:
--   local variableName = value
--
-- The print statements are already correct.
-- FIX THE VALUES so the output matches.

function declareVars()
  local lang = "???"    -- change "???" to: "Luau"
  local version = 0     -- change 0 to: 6

  print("Name: " .. lang)
  print("Version: " .. version)
end`,
    testCases: [
      { id: 1, input: "declareVars()", expectedOutput: "Name: Luau\nVersion: 6", setupCode: "declareVars()" },
      { id: 2, input: "declareVars() — run 2", expectedOutput: "Name: Luau\nVersion: 6", setupCode: "declareVars()" },
      { id: 3, input: "declareVars() — run 3", expectedOutput: "Name: Luau\nVersion: 6", setupCode: "declareVars()" },
    ],
  },

  // ── Tutorial 3 of 14: String Concatenation ───────────────────────────────
  {
    id: "string-join",
    title: "Joining Strings",
    difficulty: "easy",
    description: `── String Concatenation ─────────────────────────
In Luau, join (combine) strings with the .. operator:

  "Hello" .. ", " .. "World"   → "Hello, World"
  "Hi, " .. name               → "Hi, Alice"  (if name = "Alice")
  "Score: " .. 99              → "Score: 99"

You can chain as many pieces as you want!

── Parameters ───────────────────────────────────
When a function is called with a value like greet("Alice"),
that value is stored in the parameter name:

  function greet(name)    ← name receives "Alice"
    print(name)           → Alice
  end

── Your Task ────────────────────────────────────
Complete greet(name) so it prints:

  Hello, [name]!

Example: greet("Alice") → Hello, Alice!`,
    examples: `-- greet("Alice") → Hello, Alice!\n-- greet("World") → Hello, World!`,
    starterCode: `-- TUTORIAL 3 of 14: String Concatenation (..)
-- ─────────────────────────────────────────────
-- The .. operator joins strings:
--   "a" .. "b" .. "c"   → "abc"
--   "Hi, " .. name      → "Hi, Alice"
--
-- FIX THE BUG: Replace each ??? with the correct string.
-- The output should be: Hello, [name]!

function greet(name)
  print("???" .. name .. "???")
end`,
    testCases: [
      { id: 1, input: 'greet("Alice")', expectedOutput: "Hello, Alice!", setupCode: 'greet("Alice")' },
      { id: 2, input: 'greet("World")', expectedOutput: "Hello, World!", setupCode: 'greet("World")' },
      { id: 3, input: 'greet("Luau")', expectedOutput: "Hello, Luau!", setupCode: 'greet("Luau")' },
    ],
  },

  // ── Tutorial 4 of 14: Arithmetic Operators ───────────────────────────────
  {
    id: "arithmetic",
    title: "Doing Math",
    difficulty: "easy",
    description: `── Arithmetic Operators ─────────────────────────
Luau supports standard math operations:

  +   addition          3 + 4  = 7
  -   subtraction       10 - 3 = 7
  *   multiplication    3 * 4  = 12
  /   division          10 / 2 = 5
  %   modulo (remainder) 7 % 3  = 1

Store results in local variables, then print them:

  local sum = a + b
  print(sum)

── Your Task ────────────────────────────────────
Complete sumAndProduct(a, b) so it prints:
  Line 1: the sum of a and b
  Line 2: the product of a and b`,
    examples: `-- sumAndProduct(3, 4) prints:\n-- 7\n-- 12`,
    starterCode: `-- TUTORIAL 4 of 14: Arithmetic Operators
-- ─────────────────────────────────────────────
-- Math operators: + - * / %
-- Store results in local variables, then print them.
--
-- The sum is done for you. Add the product!

function sumAndProduct(a, b)
  local sum = a + b
  print(sum)

  local product = nil  -- FIX: replace nil with a * b
  print(product)
end`,
    testCases: [
      { id: 1, input: "sumAndProduct(3, 4)", expectedOutput: "7\n12", setupCode: "sumAndProduct(3, 4)" },
      { id: 2, input: "sumAndProduct(10, 5)", expectedOutput: "15\n50", setupCode: "sumAndProduct(10, 5)" },
      { id: 3, input: "sumAndProduct(0, 8)", expectedOutput: "8\n0", setupCode: "sumAndProduct(0, 8)" },
    ],
  },

  // ── Tutorial 5 of 14: if / then / else ───────────────────────────────────
  {
    id: "if-else",
    title: "Making Decisions",
    difficulty: "easy",
    description: `── The if Statement ─────────────────────────────
Code can make choices using if:

  if condition then
    -- runs when condition is true
  else
    -- runs when condition is false
  end

── Booleans: true and false ──────────────────────
Conditions evaluate to true or false:

  3 > 1    → true
  5 == 5   → true   (== means "equal to")
  4 == 5   → false

── The % Operator ────────────────────────────────
% gives the remainder after division:

  4 % 2 = 0   (divides evenly → even)
  7 % 2 = 1   (remainder 1 → odd)

So: n % 2 == 0 means n is even.

── Your Task ────────────────────────────────────
Complete checkEven(n):
  If n is even → print: [n] is even
  If n is odd  → print: [n] is odd`,
    examples: `-- checkEven(4) → 4 is even\n-- checkEven(7) → 7 is odd`,
    starterCode: `-- TUTORIAL 5 of 14: if / then / else
-- ─────────────────────────────────────────────
-- if checks a condition. 'then' block runs if true.
-- 'else' block runs if false.
--
-- n % 2 == 0 means n divides evenly by 2 (it's even).
--
-- ADD CODE: Complete the else branch for odd numbers.

function checkEven(n)
  if n % 2 == 0 then
    print(n .. " is even")
  else
    -- ADD CODE HERE: print "[n] is odd"

  end
end`,
    testCases: [
      { id: 1, input: "checkEven(4)", expectedOutput: "4 is even", setupCode: "checkEven(4)" },
      { id: 2, input: "checkEven(7)", expectedOutput: "7 is odd", setupCode: "checkEven(7)" },
      { id: 3, input: "checkEven(0)", expectedOutput: "0 is even", setupCode: "checkEven(0)" },
    ],
  },

  // ── Tutorial 6 of 14: if / elseif / else ─────────────────────────────────
  {
    id: "elseif",
    title: "More Choices",
    difficulty: "easy",
    description: `── elseif: Multiple Branches ─────────────────────
When you need more than two choices, add elseif:

  if condition1 then
    -- runs if condition1 is true
  elseif condition2 then
    -- runs if condition2 is true (condition1 was false)
  elseif condition3 then
    -- runs if condition3 is true
  else
    -- runs if NOTHING above was true
  end

Only ONE branch runs — the first condition that's true.

── Comparison Operators ──────────────────────────
  score >= 90   true if score is 90 or above
  score >= 80   true if score is 80 or above

Since these are checked in order, >= 80 only runs
if >= 90 was already false (so score is 80–89).

── Your Task ────────────────────────────────────
Complete grade(score) to print the letter grade:
  90 or above → A
  80 to 89    → B
  70 to 79    → C
  below 70    → F`,
    examples: `-- grade(95) → A\n-- grade(75) → C\n-- grade(60) → F`,
    starterCode: `-- TUTORIAL 6 of 14: if / elseif / else
-- ─────────────────────────────────────────────
-- Use elseif to handle more than two cases.
-- Only the FIRST matching condition runs.
--
-- ADD CODE: Complete the missing branches for C and F.

function grade(score)
  if score >= 90 then
    print("A")
  elseif score >= 80 then
    print("B")
  elseif score >= 70 then
    -- ADD CODE HERE: print "C"

  else
    -- ADD CODE HERE: print "F"

  end
end`,
    testCases: [
      { id: 1, input: "grade(95)", expectedOutput: "A", setupCode: "grade(95)" },
      { id: 2, input: "grade(75)", expectedOutput: "C", setupCode: "grade(75)" },
      { id: 3, input: "grade(60)", expectedOutput: "F", setupCode: "grade(60)" },
    ],
  },

  // ── Tutorial 7 of 14: return and Comparison Operators ────────────────────
  {
    id: "return-basics",
    title: "Returning Values",
    difficulty: "easy",
    description: `── The return Statement ─────────────────────────
A function can send a value back with return:

  function square(n)
    return n * n
  end

  print(square(4))   → 16

The caller receives whatever was returned.

── Comparison Operators ──────────────────────────
  a > b    true if a is greater than b
  a >= b   true if a is greater OR equal to b
  a < b    true if a is less than b
  a <= b   true if a is less than OR equal to b
  a == b   true if a equals b
  a ~= b   true if a does NOT equal b  (note: ~= not !=)

── Your Task ────────────────────────────────────
Complete biggerNumber(a, b) to return the larger value.
If they are equal, return either one.`,
    examples: `-- print(biggerNumber(3, 7))  → 7\n-- print(biggerNumber(10, -2)) → 10`,
    starterCode: `-- TUTORIAL 7 of 14: return and Comparison Operators
-- ─────────────────────────────────────────────
-- 'return' sends a value back to whoever called the function.
-- Use comparison operators: > < >= <= == ~=
--
-- ADD CODE: Complete the else branch to return the correct value.

function biggerNumber(a, b)
  if a >= b then
    return a
  else
    -- ADD CODE HERE: return the correct value

  end
end`,
    testCases: [
      { id: 1, input: "biggerNumber(3, 7)", expectedOutput: "7", setupCode: "print(biggerNumber(3, 7))" },
      { id: 2, input: "biggerNumber(10, -2)", expectedOutput: "10", setupCode: "print(biggerNumber(10, -2))" },
      { id: 3, input: "biggerNumber(5, 5)", expectedOutput: "5", setupCode: "print(biggerNumber(5, 5))" },
    ],
  },

  // ── Tutorial 8 of 14: The while Loop ─────────────────────────────────────
  {
    id: "while-loop",
    title: "Repeating with While",
    difficulty: "easy",
    description: `── The while Loop ───────────────────────────────
A while loop repeats code while a condition is true:

  while condition do
    -- runs over and over until condition is false
  end

Always update a variable inside, or the loop runs forever!

Example — count from 1 to 3:

  local i = 1
  while i <= 3 do
    print(i)       → 1, 2, 3
    i = i + 1      -- increase i each time (essential!)
  end

── Your Task ────────────────────────────────────
Fix the bug in countdown(n) so it prints from n down to 1.

Hint: the condition is wrong — it stops one number too early!`,
    examples: `-- countdown(3) prints:\n-- 3\n-- 2\n-- 1`,
    starterCode: `-- TUTORIAL 8 of 14: The while Loop
-- ─────────────────────────────────────────────
-- while loops repeat until the condition becomes false.
-- Update the variable each loop to avoid an infinite loop!
--
-- FIX THE BUG: The condition stops too early.
-- Change the condition so it includes 1 as well.

function countdown(n)
  local current = n

  while current > 1 do    -- BUG: should be >= 1, not > 1
    print(current)
    current = current - 1
  end
end`,
    testCases: [
      { id: 1, input: "countdown(3)", expectedOutput: "3\n2\n1", setupCode: "countdown(3)" },
      { id: 2, input: "countdown(5)", expectedOutput: "5\n4\n3\n2\n1", setupCode: "countdown(5)" },
      { id: 3, input: "countdown(1)", expectedOutput: "1", setupCode: "countdown(1)" },
    ],
  },

  // ── Tutorial 9 of 14: The Numeric for Loop ───────────────────────────────
  {
    id: "for-loop",
    title: "Counting with For",
    difficulty: "easy",
    description: `── The Numeric for Loop ─────────────────────────
The for loop counts automatically — no manual updates needed:

  for i = 1, 5 do
    print(i)   → 1, 2, 3, 4, 5
  end

i starts at 1, goes up to 5, increasing by 1 each time.

You can also specify a custom step (third number):

  for i = 0, 10, 2 do
    print(i)   → 0, 2, 4, 6, 8, 10
  end

── Your Task ────────────────────────────────────
Complete printNumbers(n) to print all numbers from 1 to n,
one per line.`,
    examples: `-- printNumbers(4) prints:\n-- 1\n-- 2\n-- 3\n-- 4`,
    starterCode: `-- TUTORIAL 9 of 14: The Numeric for Loop
-- ─────────────────────────────────────────────
-- for i = start, finish do ... end
-- i automatically counts from start to finish.
-- No need to write i = i + 1 yourself!
--
-- ADD CODE: Print the value of i inside the loop.

function printNumbers(n)
  for i = 1, n do
    -- ADD CODE HERE: print the value of i

  end
end`,
    testCases: [
      { id: 1, input: "printNumbers(4)", expectedOutput: "1\n2\n3\n4", setupCode: "printNumbers(4)" },
      { id: 2, input: "printNumbers(1)", expectedOutput: "1", setupCode: "printNumbers(1)" },
      { id: 3, input: "printNumbers(6)", expectedOutput: "1\n2\n3\n4\n5\n6", setupCode: "printNumbers(6)" },
    ],
  },

  // ── Tutorial 10 of 14: Tables ────────────────────────────────────────────
  {
    id: "tables",
    title: "Your First Table",
    difficulty: "easy",
    description: `── Tables: Lists in Luau ─────────────────────────
A table holds multiple values in order. Create one with {}:

  local colors = {"red", "green", "blue"}

Access items by position (index). Tables start at 1, not 0!

  colors[1]   → "red"
  colors[2]   → "green"
  colors[3]   → "blue"

Get the number of items with #:

  #colors   → 3

The last item is always at index #t:

  colors[#colors]   → "blue"

── Your Task ────────────────────────────────────
Read and run tableInfo(t). It's already complete!
Make all 3 tests pass by understanding the code.`,
    examples: `-- tableInfo({10, 20, 30}) prints:\n-- 10\n-- 30\n-- 3`,
    starterCode: `-- TUTORIAL 10 of 14: Tables (Lists)
-- ─────────────────────────────────────────────
-- Tables store multiple values in order.
-- Index starts at 1, not 0!
--
--   t[1]   → first element
--   t[#t]  → last element  (#t is the length)
--   #t     → total number of elements
--
-- This code is complete! Read it, run it, understand it.

function tableInfo(t)
  print(t[1])    -- first element
  print(t[#t])   -- last element
  print(#t)      -- total count
end`,
    testCases: [
      { id: 1, input: "tableInfo({10, 20, 30})", expectedOutput: "10\n30\n3", setupCode: "tableInfo({10, 20, 30})" },
      { id: 2, input: "tableInfo({99})", expectedOutput: "99\n99\n1", setupCode: "tableInfo({99})" },
      { id: 3, input: "tableInfo({1, 2, 3, 4, 5})", expectedOutput: "1\n5\n5", setupCode: "tableInfo({1, 2, 3, 4, 5})" },
    ],
  },

  // ── Tutorial 11 of 14: ipairs ────────────────────────────────────────────
  {
    id: "ipairs",
    title: "Looping Over Tables",
    difficulty: "easy",
    description: `── Iterating Tables with ipairs ─────────────────
Use ipairs() to loop over every element of a table:

  local nums = {10, 20, 30}

  for i, value in ipairs(nums) do
    print(i, value)
  end

  → 1  10
  → 2  20
  → 3  30

i is the position (1, 2, 3...) and value is the element.

If you only need the value, use _ for the index (a convention
meaning "I don't need this"):

  for _, value in ipairs(nums) do
    print(value)   → 10, 20, 30
  end

── Your Task ────────────────────────────────────
Read and run printAll(t). It's already complete!
Make all 3 tests pass by understanding the code.`,
    examples: `-- printAll({1, 2, 3}) prints:\n-- 1\n-- 2\n-- 3`,
    starterCode: `-- TUTORIAL 11 of 14: Iterating Tables with ipairs
-- ─────────────────────────────────────────────
-- ipairs(t) gives you each element in order.
-- Use _ for the index when you don't need it.
--
-- This code is complete! Read and understand the pattern.

function printAll(t)
  for _, value in ipairs(t) do
    print(value)
  end
end`,
    testCases: [
      { id: 1, input: "printAll({1, 2, 3})", expectedOutput: "1\n2\n3", setupCode: "printAll({1, 2, 3})" },
      { id: 2, input: 'printAll({"hello", "world"})', expectedOutput: "hello\nworld", setupCode: 'printAll({"hello", "world"})' },
      { id: 3, input: "printAll({42})", expectedOutput: "42", setupCode: "printAll({42})" },
    ],
  },

  // ── Tutorial 12 of 14: Accumulator Pattern ───────────────────────────────
  {
    id: "accumulator",
    title: "Adding It All Up",
    difficulty: "easy",
    description: `── The Accumulator Pattern ───────────────────────
A very common pattern: start at 0, then add each item in a loop.

  local total = 0

  for _, v in ipairs(numbers) do
    total = total + v   -- add each element
  end

  return total          -- send the final sum back

This collects (accumulates) a result across all items.

── return Reminder ───────────────────────────────
return sends a value back to the caller:

  function double(n)
    return n * 2
  end

  print(double(5))   → 10

── Your Task ────────────────────────────────────
Complete addAll(t) to return the sum of all numbers in the table.`,
    examples: `-- print(addAll({1, 2, 3})) → 6\n-- print(addAll({10, -5, 3})) → 8`,
    starterCode: `-- TUTORIAL 12 of 14: Accumulators and return
-- ─────────────────────────────────────────────
-- Start with total = 0.
-- Loop through each element and add it to total.
-- Return the final total.
--
-- FIX: Replace 0 (the second one) with the correct variable.

function addAll(t)
  local total = 0

  for _, value in ipairs(t) do
    total = total + 0   -- FIX: replace this 0 with 'value'
  end

  return total
end`,
    testCases: [
      { id: 1, input: "addAll({1, 2, 3})", expectedOutput: "6", setupCode: "print(addAll({1, 2, 3}))" },
      { id: 2, input: "addAll({10, -5, 3})", expectedOutput: "8", setupCode: "print(addAll({10, -5, 3}))" },
      { id: 3, input: "addAll({})", expectedOutput: "0", setupCode: "print(addAll({}))" },
    ],
  },

  // ── Tutorial 13 of 14: String Functions ──────────────────────────────────
  {
    id: "string-ops",
    title: "Working with Text",
    difficulty: "easy",
    description: `── String Functions ─────────────────────────────
Luau has built-in functions for working with text:

  string.upper("hello")      → "HELLO"
  string.lower("WORLD")      → "world"
  string.len("hello")        → 5    (same as #"hello")
  string.sub("hello", 1, 3)  → "hel"  (characters 1 to 3)

You can also call them directly on a string variable:

  local s = "Hello"
  s:upper()          → "HELLO"
  s:lower()          → "hello"
  s:sub(1, 3)        → "Hel"
  #s                 → 5

── Your Task ────────────────────────────────────
Read and run textInfo(s). It's already complete!
Make all 3 tests pass by understanding the code.`,
    examples: `-- textInfo("Hello") prints:\n-- HELLO\n-- hello\n-- 5`,
    starterCode: `-- TUTORIAL 13 of 14: String Functions
-- ─────────────────────────────────────────────
-- string.upper() → ALL CAPS
-- string.lower() → all lowercase
-- #s             → number of characters
--
-- This code is complete! Read and understand it.
-- Try modifying the function to experiment!

function textInfo(s)
  print(string.upper(s))  -- uppercase version
  print(string.lower(s))  -- lowercase version
  print(#s)               -- number of characters
end`,
    testCases: [
      { id: 1, input: 'textInfo("Hello")', expectedOutput: "HELLO\nhello\n5", setupCode: 'textInfo("Hello")' },
      { id: 2, input: 'textInfo("Luau")', expectedOutput: "LUAU\nluau\n4", setupCode: 'textInfo("Luau")' },
      { id: 3, input: 'textInfo("ABC")', expectedOutput: "ABC\nabc\n3", setupCode: 'textInfo("ABC")' },
    ],
  },

  // ── Tutorial 14 of 14: Combining Everything ──────────────────────────────
  {
    id: "combining",
    title: "Putting It Together",
    difficulty: "easy",
    description: `── You've Learned All the Basics! ───────────────
So far you've covered:

  ✓ print() and strings
  ✓ local variables
  ✓ string concatenation (..)
  ✓ arithmetic operators
  ✓ if / elseif / else
  ✓ while and for loops
  ✓ tables, #, and ipairs
  ✓ return values

Now combine them in one final challenge!

── Your Task ────────────────────────────────────
Complete buildReport(names) to print a team report:

  Team size: [count]
  Members:
  - [name1]
  - [name2]
  ...

Example with {"Alice", "Bob"}:

  Team size: 2
  Members:
  - Alice
  - Bob`,
    examples: `-- buildReport({"Alice", "Bob"}) prints:\n-- Team size: 2\n-- Members:\n-- - Alice\n-- - Bob`,
    starterCode: `-- TUTORIAL 14 of 14: Combining Everything
-- ─────────────────────────────────────────────
-- Use all your skills:
--   tables (#, ipairs), for loops, .. concatenation, print
--
-- ADD CODE: Complete the loop body.
-- Each name should print as "- [name]"

function buildReport(names)
  print("Team size: " .. #names)
  print("Members:")

  for _, name in ipairs(names) do
    -- ADD CODE HERE: print "- " followed by the name

  end
end`,
    testCases: [
      { id: 1, input: 'buildReport({"Alice", "Bob"})', expectedOutput: "Team size: 2\nMembers:\n- Alice\n- Bob", setupCode: 'buildReport({"Alice", "Bob"})' },
      { id: 2, input: 'buildReport({"Max"})', expectedOutput: "Team size: 1\nMembers:\n- Max", setupCode: 'buildReport({"Max"})' },
      { id: 3, input: 'buildReport({"X", "Y", "Z"})', expectedOutput: "Team size: 3\nMembers:\n- X\n- Y\n- Z", setupCode: 'buildReport({"X", "Y", "Z"})' },
    ],
  },

  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //  MEDIUM  (15 challenges — easy LeetCode level)
  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  {
    id: "fizzbuzz",
    title: "FizzBuzz",
    difficulty: "medium",
    description:
      'Hint: Use a for loop (Tutorial 9) with if/elseif (Tutorial 6) and the % operator (Tutorial 4).\n\nWrite a function `fizzbuzz(n)` that prints numbers from 1 to n.\nFor multiples of 3 print "Fizz", for multiples of 5 print "Buzz", and for multiples of both print "FizzBuzz".',
    examples: "-- fizzbuzz(5) prints:\n-- 1\n-- 2\n-- Fizz\n-- 4\n-- Buzz",
    starterCode: `function fizzbuzz(n)
  for i = 1, n do
    -- your logic here
  end
end`,
    testCases: [
      { id: 1, input: "fizzbuzz(5)", expectedOutput: "1\n2\nFizz\n4\nBuzz", setupCode: "fizzbuzz(5)" },
      { id: 2, input: "fizzbuzz(15)", expectedOutput: "1\n2\nFizz\n4\nBuzz\nFizz\n7\n8\nFizz\nBuzz\n11\nFizz\n13\n14\nFizzBuzz", setupCode: "fizzbuzz(15)" },
      { id: 3, input: "fizzbuzz(3)", expectedOutput: "1\n2\nFizz", setupCode: "fizzbuzz(3)" },
    ],
  },
  {
    id: "reverse-string",
    title: "Reverse a String",
    difficulty: "medium",
    description: "Hint: Use string.sub (Tutorial 13) and a for loop counting backwards (Tutorial 9).\n\nWrite a function `reverseStr(s)` that returns the reversed version of the input string.",
    examples: '-- reverseStr("hello") -> "olleh"\n-- reverseStr("abc") -> "cba"',
    starterCode: `function reverseStr(s)
  -- return the reversed string
end`,
    testCases: [
      { id: 1, input: 'reverseStr("hello")', expectedOutput: "olleh", setupCode: 'print(reverseStr("hello"))' },
      { id: 2, input: 'reverseStr("abc")', expectedOutput: "cba", setupCode: 'print(reverseStr("abc"))' },
      { id: 3, input: 'reverseStr("a")', expectedOutput: "a", setupCode: 'print(reverseStr("a"))' },
    ],
  },
  {
    id: "count-vowels",
    title: "Count Vowels",
    difficulty: "medium",
    description: "Hint: Loop through each character using string.sub (Tutorial 13), compare with if (Tutorial 5), and count with an accumulator (Tutorial 12).\n\nWrite a function `countVowels(s)` that returns the number of vowels (a, e, i, o, u) in the string. Case-insensitive.",
    examples: '-- countVowels("hello") -> 2\n-- countVowels("AEIOU") -> 5',
    starterCode: `function countVowels(s)
  local count = 0
  -- your logic here
  return count
end`,
    testCases: [
      { id: 1, input: 'countVowels("hello")', expectedOutput: "2", setupCode: 'print(countVowels("hello"))' },
      { id: 2, input: 'countVowels("AEIOU")', expectedOutput: "5", setupCode: 'print(countVowels("AEIOU"))' },
      { id: 3, input: 'countVowels("xyz")', expectedOutput: "0", setupCode: 'print(countVowels("xyz"))' },
    ],
  },
  {
    id: "sum-table",
    title: "Sum of Table",
    difficulty: "medium",
    description: "Hint: Use the accumulator pattern with ipairs (Tutorials 11–12). This time, figure it out on your own!\n\nWrite a function `sumTable(t)` that returns the sum of all numeric values in a table (array).",
    examples: "-- sumTable({1, 2, 3}) -> 6\n-- sumTable({10, -5, 3}) -> 8",
    starterCode: `function sumTable(t)
  local total = 0
  -- iterate and sum
  return total
end`,
    testCases: [
      { id: 1, input: "sumTable({1, 2, 3})", expectedOutput: "6", setupCode: "print(sumTable({1, 2, 3}))" },
      { id: 2, input: "sumTable({10, -5, 3})", expectedOutput: "8", setupCode: "print(sumTable({10, -5, 3}))" },
      { id: 3, input: "sumTable({})", expectedOutput: "0", setupCode: "print(sumTable({}))" },
    ],
  },
  {
    id: "factorial",
    title: "Factorial",
    difficulty: "medium",
    description: "Hint: Start with result = 1 and multiply by each number from 1 to n using a for loop (Tutorial 9). 0! = 1 by definition.\n\nWrite a function `factorial(n)` that returns n! (n factorial).\n0! = 1, 1! = 1, 5! = 120.",
    examples: "-- factorial(5) -> 120\n-- factorial(0) -> 1",
    starterCode: `function factorial(n)
  -- return n!
end`,
    testCases: [
      { id: 1, input: "factorial(5)", expectedOutput: "120", setupCode: "print(factorial(5))" },
      { id: 2, input: "factorial(0)", expectedOutput: "1", setupCode: "print(factorial(0))" },
      { id: 3, input: "factorial(7)", expectedOutput: "5040", setupCode: "print(factorial(7))" },
    ],
  },
  {
    id: "power",
    title: "Power Function",
    difficulty: "medium",
    description: "Hint: Similar to factorial — multiply base by itself, exp times, using a for loop (Tutorial 9).\n\nWrite a function `power(base, exp)` that returns base raised to the power of exp (without using `math.pow` or `^`). Assume exp >= 0.",
    examples: "-- power(2, 3) -> 8\n-- power(5, 0) -> 1",
    starterCode: `function power(base, exp)
  -- calculate base^exp manually
end`,
    testCases: [
      { id: 1, input: "power(2, 3)", expectedOutput: "8", setupCode: "print(power(2, 3))" },
      { id: 2, input: "power(5, 0)", expectedOutput: "1", setupCode: "print(power(5, 0))" },
      { id: 3, input: "power(3, 4)", expectedOutput: "81", setupCode: "print(power(3, 4))" },
    ],
  },
  {
    id: "find-min",
    title: "Find Minimum in Table",
    difficulty: "medium",
    description: "Hint: Start with min = t[1] (Tutorial 10), then compare each element with if (Tutorial 5). Update min when you find something smaller.\n\nWrite a function `findMin(t)` that returns the smallest number in a table (array). Assume the table has at least one element.",
    examples: "-- findMin({3, 1, 4, 1, 5}) -> 1\n-- findMin({-2, -8, -1}) -> -8",
    starterCode: `function findMin(t)
  -- find and return the minimum
end`,
    testCases: [
      { id: 1, input: "findMin({3, 1, 4, 1, 5})", expectedOutput: "1", setupCode: "print(findMin({3, 1, 4, 1, 5}))" },
      { id: 2, input: "findMin({-2, -8, -1})", expectedOutput: "-8", setupCode: "print(findMin({-2, -8, -1}))" },
      { id: 3, input: "findMin({42})", expectedOutput: "42", setupCode: "print(findMin({42}))" },
    ],
  },
  {
    id: "count-char",
    title: "Count Character",
    difficulty: "medium",
    description: "Hint: Loop through each character using string.sub (Tutorial 13) and an accumulator (Tutorial 12). Compare with == (Tutorial 7).\n\nWrite a function `countChar(s, ch)` that returns how many times the character `ch` appears in string `s`.",
    examples: '-- countChar("banana", "a") -> 3\n-- countChar("hello", "z") -> 0',
    starterCode: `function countChar(s, ch)
  local count = 0
  -- your logic here
  return count
end`,
    testCases: [
      { id: 1, input: 'countChar("banana", "a")', expectedOutput: "3", setupCode: 'print(countChar("banana", "a"))' },
      { id: 2, input: 'countChar("hello", "z")', expectedOutput: "0", setupCode: 'print(countChar("hello", "z"))' },
      { id: 3, input: 'countChar("aaa", "a")', expectedOutput: "3", setupCode: 'print(countChar("aaa", "a"))' },
    ],
  },
  {
    id: "is-prime",
    title: "Is Prime",
    difficulty: "medium",
    description: "Hint: A number is prime if no integer from 2 to n-1 divides it evenly (% operator from Tutorial 4). Use a for loop (Tutorial 9) and return false as soon as you find a divisor.\n\nWrite a function `isPrime(n)` that returns `true` if n is a prime number, `false` otherwise. Assume n >= 1.",
    examples: "-- isPrime(7) -> true\n-- isPrime(4) -> false\n-- isPrime(1) -> false",
    starterCode: `function isPrime(n)
  -- check if n is prime
end`,
    testCases: [
      { id: 1, input: "isPrime(7)", expectedOutput: "true", setupCode: "print(tostring(isPrime(7)))" },
      { id: 2, input: "isPrime(4)", expectedOutput: "false", setupCode: "print(tostring(isPrime(4)))" },
      { id: 3, input: "isPrime(1)", expectedOutput: "false", setupCode: "print(tostring(isPrime(1)))" },
    ],
  },
  {
    id: "title-case",
    title: "Title Case",
    difficulty: "medium",
    description: 'Hint: Use string.upper for the first character and string.sub for the rest (Tutorial 13). Split words by looping through the string.\n\nWrite a function `titleCase(s)` that capitalizes the first letter of each word in a string. Words are separated by spaces.',
    examples: '-- titleCase("hello world") -> "Hello World"\n-- titleCase("luau is fun") -> "Luau Is Fun"',
    starterCode: `function titleCase(s)
  -- capitalize first letter of each word
end`,
    testCases: [
      { id: 1, input: 'titleCase("hello world")', expectedOutput: "Hello World", setupCode: 'print(titleCase("hello world"))' },
      { id: 2, input: 'titleCase("luau is fun")', expectedOutput: "Luau Is Fun", setupCode: 'print(titleCase("luau is fun"))' },
      { id: 3, input: 'titleCase("a")', expectedOutput: "A", setupCode: 'print(titleCase("a"))' },
    ],
  },
  {
    id: "repeat-string",
    title: "Repeat String",
    difficulty: "medium",
    description: "Hint: Use string concatenation (..) inside a for loop (Tutorials 3 and 9). Start with an empty string and append s each iteration.\n\nWrite a function `repeatStr(s, n)` that returns the string `s` repeated `n` times concatenated together.",
    examples: '-- repeatStr("ab", 3) -> "ababab"\n-- repeatStr("x", 5) -> "xxxxx"',
    starterCode: `function repeatStr(s, n)
  -- return s repeated n times
end`,
    testCases: [
      { id: 1, input: 'repeatStr("ab", 3)', expectedOutput: "ababab", setupCode: 'print(repeatStr("ab", 3))' },
      { id: 2, input: 'repeatStr("x", 5)', expectedOutput: "xxxxx", setupCode: 'print(repeatStr("x", 5))' },
      { id: 3, input: 'repeatStr("hi", 1)', expectedOutput: "hi", setupCode: 'print(repeatStr("hi", 1))' },
    ],
  },
  {
    id: "find-index",
    title: "Find Index",
    difficulty: "medium",
    description: "Hint: Use ipairs (Tutorial 11) — this time you need the index i too (not just value). Return i when value matches, or -1 if not found.\n\nWrite a function `findIndex(t, value)` that returns the index of `value` in the table, or -1 if not found. Tables are 1-indexed in Luau.",
    examples: "-- findIndex({10, 20, 30}, 20) -> 2\n-- findIndex({1, 2, 3}, 5) -> -1",
    starterCode: `function findIndex(t, value)
  -- return index or -1
end`,
    testCases: [
      { id: 1, input: "findIndex({10, 20, 30}, 20)", expectedOutput: "2", setupCode: "print(findIndex({10, 20, 30}, 20))" },
      { id: 2, input: "findIndex({1, 2, 3}, 5)", expectedOutput: "-1", setupCode: "print(findIndex({1, 2, 3}, 5))" },
      { id: 3, input: "findIndex({5, 5, 5}, 5)", expectedOutput: "1", setupCode: "print(findIndex({5, 5, 5}, 5))" },
    ],
  },
  {
    id: "join-strings",
    title: "Join Table",
    difficulty: "medium",
    description: 'Hint: Use ipairs (Tutorial 11) and .. concatenation (Tutorial 3). Be careful not to add the separator after the last element.\n\nWrite a function `joinTable(t, sep)` that joins all elements of a string table with the separator `sep`.',
    examples: '-- joinTable({"a", "b", "c"}, "-") -> "a-b-c"\n-- joinTable({"hello", "world"}, " ") -> "hello world"',
    starterCode: `function joinTable(t, sep)
  -- join elements with separator
end`,
    testCases: [
      { id: 1, input: 'joinTable({"a","b","c"}, "-")', expectedOutput: "a-b-c", setupCode: 'print(joinTable({"a","b","c"}, "-"))' },
      { id: 2, input: 'joinTable({"hello","world"}, " ")', expectedOutput: "hello world", setupCode: 'print(joinTable({"hello","world"}, " "))' },
      { id: 3, input: 'joinTable({"x"}, ",")', expectedOutput: "x", setupCode: 'print(joinTable({"x"}, ","))' },
    ],
  },
  {
    id: "double-each",
    title: "Double Each Element",
    difficulty: "medium",
    description: "Hint: Use ipairs (Tutorial 11) and multiply each value by 2 (Tutorial 4). Print each result.\n\nWrite a function `doubleEach(t)` that prints each element of the table multiplied by 2, one per line.",
    examples: "-- doubleEach({1, 2, 3}) prints:\n-- 2\n-- 4\n-- 6",
    starterCode: `function doubleEach(t)
  -- print each element * 2
end`,
    testCases: [
      { id: 1, input: "doubleEach({1, 2, 3})", expectedOutput: "2\n4\n6", setupCode: "doubleEach({1, 2, 3})" },
      { id: 2, input: "doubleEach({0, -5, 10})", expectedOutput: "0\n-10\n20", setupCode: "doubleEach({0, -5, 10})" },
      { id: 3, input: "doubleEach({7})", expectedOutput: "14", setupCode: "doubleEach({7})" },
    ],
  },
  {
    id: "sum-digits",
    title: "Sum of Digits",
    difficulty: "medium",
    description: "Hint: Use % 10 to extract the last digit, then divide by 10 (with math.floor) to remove it. Repeat with a while loop (Tutorial 8) until n reaches 0.\n\nWrite a function `sumDigits(n)` that returns the sum of all digits of a positive integer.",
    examples: "-- sumDigits(123) -> 6   (1+2+3)\n-- sumDigits(9) -> 9",
    starterCode: `function sumDigits(n)
  -- sum each digit of n
end`,
    testCases: [
      { id: 1, input: "sumDigits(123)", expectedOutput: "6", setupCode: "print(sumDigits(123))" },
      { id: 2, input: "sumDigits(9)", expectedOutput: "9", setupCode: "print(sumDigits(9))" },
      { id: 3, input: "sumDigits(9999)", expectedOutput: "36", setupCode: "print(sumDigits(9999))" },
    ],
  },

  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //  HARD  (11 challenges — medium LeetCode level)
  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  {
    id: "fibonacci",
    title: "Fibonacci Sequence",
    difficulty: "hard",
    description: "Write a function `fibonacci(n)` that prints the first `n` numbers of the Fibonacci sequence, one per line. The sequence starts with 0, 1, 1, 2, 3, 5, ...",
    examples: "-- fibonacci(5) prints:\n-- 0\n-- 1\n-- 1\n-- 2\n-- 3",
    starterCode: `function fibonacci(n)
  -- print first n fibonacci numbers
end`,
    testCases: [
      { id: 1, input: "fibonacci(5)", expectedOutput: "0\n1\n1\n2\n3", setupCode: "fibonacci(5)" },
      { id: 2, input: "fibonacci(1)", expectedOutput: "0", setupCode: "fibonacci(1)" },
      { id: 3, input: "fibonacci(8)", expectedOutput: "0\n1\n1\n2\n3\n5\n8\n13", setupCode: "fibonacci(8)" },
    ],
  },
  {
    id: "palindrome",
    title: "Palindrome Check",
    difficulty: "hard",
    description: 'Write a function `isPalindrome(s)` that returns `true` if the string reads the same forwards and backwards (case-insensitive), `false` otherwise.',
    examples: '-- isPalindrome("racecar") -> true\n-- isPalindrome("hello") -> false\n-- isPalindrome("Aba") -> true',
    starterCode: `function isPalindrome(s)
  -- check if s is a palindrome
end`,
    testCases: [
      { id: 1, input: 'isPalindrome("racecar")', expectedOutput: "true", setupCode: 'print(tostring(isPalindrome("racecar")))' },
      { id: 2, input: 'isPalindrome("hello")', expectedOutput: "false", setupCode: 'print(tostring(isPalindrome("hello")))' },
      { id: 3, input: 'isPalindrome("Aba")', expectedOutput: "true", setupCode: 'print(tostring(isPalindrome("Aba")))' },
    ],
  },
  {
    id: "gcd",
    title: "Greatest Common Divisor",
    difficulty: "hard",
    description: "Write a function `gcd(a, b)` that returns the greatest common divisor of two positive integers using the Euclidean algorithm.",
    examples: "-- gcd(12, 8) -> 4\n-- gcd(17, 5) -> 1\n-- gcd(100, 75) -> 25",
    starterCode: `function gcd(a, b)
  -- Euclidean algorithm
end`,
    testCases: [
      { id: 1, input: "gcd(12, 8)", expectedOutput: "4", setupCode: "print(gcd(12, 8))" },
      { id: 2, input: "gcd(17, 5)", expectedOutput: "1", setupCode: "print(gcd(17, 5))" },
      { id: 3, input: "gcd(100, 75)", expectedOutput: "25", setupCode: "print(gcd(100, 75))" },
    ],
  },
  {
    id: "bubble-sort",
    title: "Bubble Sort",
    difficulty: "hard",
    description: "Write a function `bubbleSort(t)` that sorts a table of numbers in ascending order using bubble sort, then prints each element one per line.",
    examples: "-- bubbleSort({3, 1, 2}) prints:\n-- 1\n-- 2\n-- 3",
    starterCode: `function bubbleSort(t)
  -- sort the table, then print each element
end`,
    testCases: [
      { id: 1, input: "bubbleSort({3, 1, 2})", expectedOutput: "1\n2\n3", setupCode: "bubbleSort({3, 1, 2})" },
      { id: 2, input: "bubbleSort({5, -1, 4, 0})", expectedOutput: "-1\n0\n4\n5", setupCode: "bubbleSort({5, -1, 4, 0})" },
      { id: 3, input: "bubbleSort({1})", expectedOutput: "1", setupCode: "bubbleSort({1})" },
    ],
  },
  {
    id: "caesar-cipher",
    title: "Caesar Cipher",
    difficulty: "hard",
    description: "Write a function `caesarEncrypt(s, shift)` that shifts each lowercase letter by `shift` positions in the alphabet, wrapping around. Non-lowercase characters remain unchanged.",
    examples: '-- caesarEncrypt("abc", 1) -> "bcd"\n-- caesarEncrypt("xyz", 3) -> "abc"',
    starterCode: `function caesarEncrypt(s, shift)
  -- shift each lowercase letter
end`,
    testCases: [
      { id: 1, input: 'caesarEncrypt("abc", 1)', expectedOutput: "bcd", setupCode: 'print(caesarEncrypt("abc", 1))' },
      { id: 2, input: 'caesarEncrypt("xyz", 3)', expectedOutput: "abc", setupCode: 'print(caesarEncrypt("xyz", 3))' },
      { id: 3, input: 'caesarEncrypt("hello", 13)', expectedOutput: "uryyb", setupCode: 'print(caesarEncrypt("hello", 13))' },
    ],
  },
  {
    id: "two-sum",
    title: "Two Sum",
    difficulty: "hard",
    description: "Write a function `twoSum(t, target)` that finds two numbers in the table that add up to `target` and prints their indices (1-based), separated by a space. If no pair exists, print -1. Return the first valid pair found (smallest first index).",
    examples: '-- twoSum({2, 7, 11, 15}, 9) -> "1 2"\n-- twoSum({1, 2, 3}, 10) -> "-1"',
    starterCode: `function twoSum(t, target)
  -- find indices of two numbers that sum to target
end`,
    testCases: [
      { id: 1, input: "twoSum({2, 7, 11, 15}, 9)", expectedOutput: "1 2", setupCode: "twoSum({2, 7, 11, 15}, 9)" },
      { id: 2, input: "twoSum({1, 2, 3}, 10)", expectedOutput: "-1", setupCode: "twoSum({1, 2, 3}, 10)" },
      { id: 3, input: "twoSum({3, 5, 1, 4}, 9)", expectedOutput: "2 4", setupCode: "twoSum({3, 5, 1, 4}, 9)" },
    ],
  },
  {
    id: "flatten-table",
    title: "Flatten Nested Table",
    difficulty: "hard",
    description: "Write a function `flatten(t)` that takes a nested table (up to 2 levels deep) and prints all values in order, one per line. Sub-tables should be expanded inline.",
    examples: "-- flatten({1, {2, 3}, 4}) prints:\n-- 1\n-- 2\n-- 3\n-- 4",
    starterCode: `function flatten(t)
  -- print all values, expanding sub-tables
end`,
    testCases: [
      { id: 1, input: "flatten({1, {2, 3}, 4})", expectedOutput: "1\n2\n3\n4", setupCode: "flatten({1, {2, 3}, 4})" },
      { id: 2, input: "flatten({{1, 2}, {3, 4}})", expectedOutput: "1\n2\n3\n4", setupCode: "flatten({{1, 2}, {3, 4}})" },
      { id: 3, input: "flatten({5})", expectedOutput: "5", setupCode: "flatten({5})" },
    ],
  },
  {
    id: "anagram-check",
    title: "Anagram Check",
    difficulty: "hard",
    description: 'Write a function `isAnagram(a, b)` that returns `true` if strings `a` and `b` are anagrams of each other (same characters, same count, case-insensitive), `false` otherwise.',
    examples: '-- isAnagram("listen", "silent") -> true\n-- isAnagram("hello", "world") -> false',
    starterCode: `function isAnagram(a, b)
  -- check if a and b are anagrams
end`,
    testCases: [
      { id: 1, input: 'isAnagram("listen", "silent")', expectedOutput: "true", setupCode: 'print(tostring(isAnagram("listen", "silent")))' },
      { id: 2, input: 'isAnagram("hello", "world")', expectedOutput: "false", setupCode: 'print(tostring(isAnagram("hello", "world")))' },
      { id: 3, input: 'isAnagram("Astronomer", "Moon starer")', expectedOutput: "false", setupCode: 'print(tostring(isAnagram("Astronomer", "Moon starer")))' },
    ],
  },
  {
    id: "run-length-encoding",
    title: "Run-Length Encoding",
    difficulty: "hard",
    description: 'Write a function `rle(s)` that compresses a string using run-length encoding. Consecutive identical characters are replaced by the character followed by its count.',
    examples: '-- rle("aaabbc") -> "a3b2c1"\n-- rle("aabba") -> "a2b2a1"',
    starterCode: `function rle(s)
  -- return run-length encoded string
end`,
    testCases: [
      { id: 1, input: 'rle("aaabbc")', expectedOutput: "a3b2c1", setupCode: 'print(rle("aaabbc"))' },
      { id: 2, input: 'rle("aabba")', expectedOutput: "a2b2a1", setupCode: 'print(rle("aabba"))' },
      { id: 3, input: 'rle("x")', expectedOutput: "x1", setupCode: 'print(rle("x"))' },
    ],
  },
  {
    id: "matrix-sum",
    title: "Matrix Diagonal Sum",
    difficulty: "hard",
    description: "Write a function `diagSum(m)` that returns the sum of the main diagonal of a square matrix (table of tables). The main diagonal consists of elements where row index equals column index.",
    examples: "-- diagSum({{1,2,3},{4,5,6},{7,8,9}}) -> 15  (1+5+9)",
    starterCode: `function diagSum(m)
  -- sum the main diagonal
end`,
    testCases: [
      { id: 1, input: "diagSum({{1,2,3},{4,5,6},{7,8,9}})", expectedOutput: "15", setupCode: "print(diagSum({{1,2,3},{4,5,6},{7,8,9}}))" },
      { id: 2, input: "diagSum({{5}})", expectedOutput: "5", setupCode: "print(diagSum({{5}}))" },
      { id: 3, input: "diagSum({{1,0},{0,1}})", expectedOutput: "2", setupCode: "print(diagSum({{1,0},{0,1}}))" },
    ],
  },
  {
    id: "merge-sorted",
    title: "Merge Sorted Tables",
    difficulty: "hard",
    description: "Write a function `mergeSorted(a, b)` that merges two already-sorted tables into a single sorted table and prints each element one per line.",
    examples: "-- mergeSorted({1, 3, 5}, {2, 4, 6}) prints:\n-- 1\n-- 2\n-- 3\n-- 4\n-- 5\n-- 6",
    starterCode: `function mergeSorted(a, b)
  -- merge two sorted tables and print
end`,
    testCases: [
      { id: 1, input: "mergeSorted({1,3,5},{2,4,6})", expectedOutput: "1\n2\n3\n4\n5\n6", setupCode: "mergeSorted({1,3,5},{2,4,6})" },
      { id: 2, input: "mergeSorted({1,2},{3,4,5})", expectedOutput: "1\n2\n3\n4\n5", setupCode: "mergeSorted({1,2},{3,4,5})" },
      { id: 3, input: "mergeSorted({},{1,2})", expectedOutput: "1\n2", setupCode: "mergeSorted({},{1,2})" },
    ],
  },
];
