import numpy as np
from PIL import Image, ImageDraw, ImageFont
import os

# Define element data: symbol, name, category for coloring
elements = [
    # Format: (atomic_number, symbol, name, category)
    (1, "H", "Hydrogen", "nonmetal"),
    (2, "He", "Helium", "noble gas"),
    # ... Add all other elements here
]

# Complete the element list
elements = [
    (1, "H", "Hydrogen", "nonmetal"),
    (2, "He", "Helium", "noble gas"),
    (3, "Li", "Lithium", "alkali metal"),
    (4, "Be", "Beryllium", "alkaline earth metal"),
    (5, "B", "Boron", "metalloid"),
    (6, "C", "Carbon", "nonmetal"),
    (7, "N", "Nitrogen", "nonmetal"),
    (8, "O", "Oxygen", "nonmetal"),
    (9, "F", "Fluorine", "halogen"),
    (10, "Ne", "Neon", "noble gas"),
    (11, "Na", "Sodium", "alkali metal"),
    (12, "Mg", "Magnesium", "alkaline earth metal"),
    (13, "Al", "Aluminium", "post-transition metal"),
    (14, "Si", "Silicon", "metalloid"),
    (15, "P", "Phosphorus", "nonmetal"),
    (16, "S", "Sulfur", "nonmetal"),
    (17, "Cl", "Chlorine", "halogen"),
    (18, "Ar", "Argon", "noble gas"),
    (19, "K", "Potassium", "alkali metal"),
    (20, "Ca", "Calcium", "alkaline earth metal"),
    (21, "Sc", "Scandium", "transition metal"),
    (22, "Ti", "Titanium", "transition metal"),
    (23, "V", "Vanadium", "transition metal"),
    (24, "Cr", "Chromium", "transition metal"),
    (25, "Mn", "Manganese", "transition metal"),
    (26, "Fe", "Iron", "transition metal"),
    (27, "Co", "Cobalt", "transition metal"),
    (28, "Ni", "Nickel", "transition metal"),
    (29, "Cu", "Copper", "transition metal"),
    (30, "Zn", "Zinc", "transition metal"),
    (31, "Ga", "Gallium", "post-transition metal"),
    (32, "Ge", "Germanium", "metalloid"),
    (33, "As", "Arsenic", "metalloid"),
    (34, "Se", "Selenium", "nonmetal"),
    (35, "Br", "Bromine", "halogen"),
    (36, "Kr", "Krypton", "noble gas"),
    (37, "Rb", "Rubidium", "alkali metal"),
    (38, "Sr", "Strontium", "alkaline earth metal"),
    (39, "Y", "Yttrium", "transition metal"),
    (40, "Zr", "Zirconium", "transition metal"),
    (41, "Nb", "Niobium", "transition metal"),
    (42, "Mo", "Molybdenum", "transition metal"),
    (43, "Tc", "Technetium", "transition metal"),
    (44, "Ru", "Ruthenium", "transition metal"),
    (45, "Rh", "Rhodium", "transition metal"),
    (46, "Pd", "Palladium", "transition metal"),
    (47, "Ag", "Silver", "transition metal"),
    (48, "Cd", "Cadmium", "transition metal"),
    (49, "In", "Indium", "post-transition metal"),
    (50, "Sn", "Tin", "post-transition metal"),
    (51, "Sb", "Antimony", "metalloid"),
    (52, "Te", "Tellurium", "metalloid"),
    (53, "I", "Iodine", "halogen"),
    (54, "Xe", "Xenon", "noble gas"),
    (55, "Cs", "Caesium", "alkali metal"),
    (56, "Ba", "Barium", "alkaline earth metal"),
    (57, "La", "Lanthanum", "lanthanide"),
    (58, "Ce", "Cerium", "lanthanide"),
    (59, "Pr", "Praseodymium", "lanthanide"),
    (60, "Nd", "Neodymium", "lanthanide"),
    (61, "Pm", "Promethium", "lanthanide"),
    (62, "Sm", "Samarium", "lanthanide"),
    (63, "Eu", "Europium", "lanthanide"),
    (64, "Gd", "Gadolinium", "lanthanide"),
    (65, "Tb", "Terbium", "lanthanide"),
    (66, "Dy", "Dysprosium", "lanthanide"),
    (67, "Ho", "Holmium", "lanthanide"),
    (68, "Er", "Erbium", "lanthanide"),
    (69, "Tm", "Thulium", "lanthanide"),
    (70, "Yb", "Ytterbium", "lanthanide"),
    (71, "Lu", "Lutetium", "lanthanide"),
    (72, "Hf", "Hafnium", "transition metal"),
    (73, "Ta", "Tantalum", "transition metal"),
    (74, "W", "Tungsten", "transition metal"),
    (75, "Re", "Rhenium", "transition metal"),
    (76, "Os", "Osmium", "transition metal"),
    (77, "Ir", "Iridium", "transition metal"),
    (78, "Pt", "Platinum", "transition metal"),
    (79, "Au", "Gold", "transition metal"),
    (80, "Hg", "Mercury", "transition metal"),
    (81, "Tl", "Thallium", "post-transition metal"),
    (82, "Pb", "Lead", "post-transition metal"),
    (83, "Bi", "Bismuth", "post-transition metal"),
    (84, "Po", "Polonium", "post-transition metal"),
    (85, "At", "Astatine", "halogen"),
    (86, "Rn", "Radon", "noble gas"),
    (87, "Fr", "Francium", "alkali metal"),
    (88, "Ra", "Radium", "alkaline earth metal"),
    (89, "Ac", "Actinium", "actinide"),
    (90, "Th", "Thorium", "actinide"),
    (91, "Pa", "Protactinium", "actinide"),
    (92, "U", "Uranium", "actinide"),
    (93, "Np", "Neptunium", "actinide"),
    (94, "Pu", "Plutonium", "actinide"),
    (95, "Am", "Americium", "actinide"),
    (96, "Cm", "Curium", "actinide"),
    (97, "Bk", "Berkelium", "actinide"),
    (98, "Cf", "Californium", "actinide"),
    (99, "Es", "Einsteinium", "actinide"),
    (100, "Fm", "Fermium", "actinide"),
    (101, "Md", "Mendelevium", "actinide"),
    (102, "No", "Nobelium", "actinide"),
    (103, "Lr", "Lawrencium", "actinide"),
    (104, "Rf", "Rutherfordium", "transition metal"),
    (105, "Db", "Dubnium", "transition metal"),
    (106, "Sg", "Seaborgium", "transition metal"),
    (107, "Bh", "Bohrium", "transition metal"),
    (108, "Hs", "Hassium", "transition metal"),
    (109, "Mt", "Meitnerium", "unknown"),
    (110, "Ds", "Darmstadtium", "unknown"),
    (111, "Rg", "Roentgenium", "unknown"),
    (112, "Cn", "Copernicium", "transition metal"),
    (113, "Nh", "Nihonium", "unknown"),
    (114, "Fl", "Flerovium", "unknown"),
    (115, "Mc", "Moscovium", "unknown"),
    (116, "Lv", "Livermorium", "unknown"),
    (117, "Ts", "Tennessine", "unknown"),
    (118, "Og", "Oganesson", "unknown"),
]

# Color scheme for different element categories
color_scheme = {
    "alkali metal": (255, 102, 102),       # Red
    "alkaline earth metal": (255, 191, 128), # Light Orange
    "transition metal": (255, 217, 102),    # Yellow
    "post-transition metal": (166, 217, 115), # Light Green
    "metalloid": (102, 204, 153),          # Sea Green
    "nonmetal": (102, 178, 255),           # Light Blue
    "halogen": (191, 128, 255),            # Light Purple
    "noble gas": (230, 128, 255),          # Pink
    "lanthanide": (255, 153, 204),         # Light Pink
    "actinide": (255, 102, 102),           # Red
    "unknown": (200, 200, 200),            # Grey
}

# Define periodic table layout
table_layout = [
    # Format: (atomic_number, row, col) - using 0-indexing
    # Main block
    (1, 0, 0), (2, 0, 17),
    (3, 1, 0), (4, 1, 1), (5, 1, 12), (6, 1, 13), (7, 1, 14), (8, 1, 15), (9, 1, 16), (10, 1, 17),
    (11, 2, 0), (12, 2, 1), (13, 2, 12), (14, 2, 13), (15, 2, 14), (16, 2, 15), (17, 2, 16), (18, 2, 17),
    (19, 3, 0), (20, 3, 1), (21, 3, 2), (22, 3, 3), (23, 3, 4), (24, 3, 5), (25, 3, 6), (26, 3, 7),
    (27, 3, 8), (28, 3, 9), (29, 3, 10), (30, 3, 11), (31, 3, 12), (32, 3, 13), (33, 3, 14), (34, 3, 15),
    (35, 3, 16), (36, 3, 17),
    (37, 4, 0), (38, 4, 1), (39, 4, 2), (40, 4, 3), (41, 4, 4), (42, 4, 5), (43, 4, 6), (44, 4, 7),
    (45, 4, 8), (46, 4, 9), (47, 4, 10), (48, 4, 11), (49, 4, 12), (50, 4, 13), (51, 4, 14), (52, 4, 15),
    (53, 4, 16), (54, 4, 17),
    (55, 5, 0), (56, 5, 1), (71, 5, 2), (72, 5, 3), (73, 5, 4), (74, 5, 5), (75, 5, 6), (76, 5, 7),
    (77, 5, 8), (78, 5, 9), (79, 5, 10), (80, 5, 11), (81, 5, 12), (82, 5, 13), (83, 5, 14), (84, 5, 15),
    (85, 5, 16), (86, 5, 17),
    (87, 6, 0), (88, 6, 1), (103, 6, 2), (104, 6, 3), (105, 6, 4), (106, 6, 5), (107, 6, 6), (108, 6, 7),
    (109, 6, 8), (110, 6, 9), (111, 6, 10), (112, 6, 11), (113, 6, 12), (114, 6, 13), (115, 6, 14), (116, 6, 15),
    (117, 6, 16), (118, 6, 17),
    
    # Lanthanides and Actinides
    (57, 8, 2), (58, 8, 3), (59, 8, 4), (60, 8, 5), (61, 8, 6), (62, 8, 7), (63, 8, 8), (64, 8, 9),
    (65, 8, 10), (66, 8, 11), (67, 8, 12), (68, 8, 13), (69, 8, 14), (70, 8, 15), (71, 8, 16),
    (89, 9, 2), (90, 9, 3), (91, 9, 4), (92, 9, 5), (93, 9, 6), (94, 9, 7), (95, 9, 8), (96, 9, 9),
    (97, 9, 10), (98, 9, 11), (99, 9, 12), (100, 9, 13), (101, 9, 14), (102, 9, 15), (103, 9, 16),
]

def create_element_texture(atomic_number, symbol, name, category):
    """Create a 128x128 texture for an element"""
    size = 128
    img = Image.new('RGB', (size, size), color_scheme[category])
    draw = ImageDraw.Draw(img)
    
    try:
        # Use a font that's available on most systems
        large_font = ImageFont.truetype("arial.ttf", 48)
        small_font = ImageFont.truetype("arial.ttf", 16)
        number_font = ImageFont.truetype("arial.ttf", 20)
    except IOError:
        # Fallback to default font
        large_font = ImageFont.load_default()
        small_font = ImageFont.load_default()
        number_font = ImageFont.load_default()
    
    # Draw atomic number
    draw.text((10, 10), str(atomic_number), fill=(0, 0, 0), font=number_font)
    
    # Draw symbol
    symbol_width = draw.textlength(symbol, font=large_font)
    draw.text(((size-symbol_width)/2, 40), symbol, fill=(0, 0, 0), font=large_font)
    
    # Draw name
    name_width = draw.textlength(name, font=small_font)
    draw.text(((size-name_width)/2, 100), name, fill=(0, 0, 0), font=small_font)
    
    # Add border
    draw.rectangle([(0, 0), (size-1, size-1)], outline=(0, 0, 0), width=2)
    
    return img

def main():
    # 1. Create all individual textures and arrange in a square layout (11x11)
    grid_size = 11  # 11x11 grid
    texture_square = Image.new('RGB', (128 * grid_size, 128 * grid_size), (240, 240, 240))
    
    for atomic_number, symbol, name, category in elements:
        if atomic_number <= 118:  # Ensure we don't go beyond known elements
            # Calculate position in the grid
            row = (atomic_number - 1) // grid_size
            col = (atomic_number - 1) % grid_size
            
            # Create simple colored square for this element (no text/borders)
            size = 128
            texture = Image.new('RGB', (size, size), color_scheme[category])
            
            # Add to the texture square at calculated position
            x_position = col * 128
            y_position = row * 128
            texture_square.paste(texture, (x_position, y_position))
    
    # Save the texture square
    texture_square.save("texture.png")
    
    # 2. Create periodic table
    # Find dimensions needed for the periodic table
    rows = 10  # 7 main rows + gap + 2 rows for lanthanides and actinides
    cols = 18
    cell_size = 128
    
    periodic_table = Image.new('RGB', (cols * cell_size, rows * cell_size), (240, 240, 240))
    
    # Place elements in the periodic table
    for atomic_number, row, col in table_layout:
        # Find the element data
        element_data = next((e for e in elements if e[0] == atomic_number), None)
        if element_data:
            _, symbol, name, category = element_data
            texture = create_element_texture(atomic_number, symbol, name, category)
            
            # Place the texture in the right position
            x_position = col * cell_size
            y_position = row * cell_size
            periodic_table.paste(texture, (x_position, y_position))
    
    # Save the periodic table
    periodic_table.save("periodic_table.png")
    
    print("Generated texture.png and periodic_table.png ")

if __name__ == "__main__":
    main()