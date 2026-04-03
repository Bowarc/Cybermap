/*
    for mental comprehension
    it can be visualized like that (numbers are indexes)
    [
        [0 , 1 , 2 , 3 ], // this is a row
        [4 , 5 , 6 , 7 ],
        [8 , 9 , 10, 11],
        [12, 13, 14, 15],
        [16, 17, 18, 19],
        [20, 21, 22, 23],
    ]
    + making a list of columns doesn't make any sense
*/
#[derive(Clone, Copy, Debug)]
pub struct IndexOutOfBoundsError;

#[derive(Debug, Clone)]
pub struct Vec2D<T> {
    elems: Box<[T]>,
    width: usize,
    height: usize,
}

impl<T: Clone + std::fmt::Debug> Vec2D<T> {
    pub fn new_from_element(elem: T, width: usize, height: usize) -> Self {
        Self {
            elems: vec![elem; width * height].into(),
            width,
            height,
        }
    }
}

impl<T> Vec2D<T> {
    pub fn new_empty() -> Self {
        Self {
            elems: Box::new([]),
            width: 0,
            height: 0,
        }
    }
    pub fn new_from_vec(base: Vec<T>, width: usize, height: usize) -> Option<Vec2D<T>> {
        if (width * height) as u64 != base.len() as u64 {
            return None;
        }

        Some(Self {
            elems: base.into(),
            width,
            height,
        })
    }
    pub fn elems(&self) -> &[T] {
        &self.elems
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn len(&self) -> usize {
        self.width * self.height
    }

    pub fn index_from_xy(&self, x: usize, y: usize) -> u64 {
        (y * self.width + x) as u64
    }

    pub fn contains_xy(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }
    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        if !self.contains_xy(x, y) {
            return None;
        }

        let index = self.index_from_xy(x, y);
        Some(&self.elems[index as usize])
    }
    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        if !self.contains_xy(x, y) {
            return None;
        }

        let index = self.index_from_xy(x, y);
        Some(&mut self.elems[index as usize])
    }
    // returns Ok(T) if it succesfully replaced an item
    // return Err(()) if it failled to place the item in the elems list
    pub fn set(&mut self, x: usize, y: usize, elem: T) -> Result<T, IndexOutOfBoundsError> {
        if !self.contains_xy(x, y) {
            return Err(IndexOutOfBoundsError);
        }

        let index = self.index_from_xy(x, y);
        Ok(std::mem::replace(
            self.elems.get_mut(index as usize).unwrap(),
            elem,
        ))
    }

    pub fn iter(&self) -> Vec2DIterator {
        Vec2DIterator {
            current_index: 0,
            width: self.width,
            height: self.height,
        }
    }
}

impl<T: PartialEq> std::cmp::PartialEq for Vec2D<T> {
    fn eq(&self, other: &Self) -> bool {
        self.elems == other.elems && self.width == other.width && self.height == other.height
    }
}

#[derive(Debug, PartialEq)]
pub struct Vec2DIterator {
    current_index: usize,
    width: usize,
    height: usize,
}

impl Vec2DIterator {
    pub fn new_from_xy_wh(x: usize, y: usize, w: usize, h: usize) -> Self {
        Self {
            current_index: y * w + x,
            width: w,
            height: h,
        }
    }
}

impl Iterator for Vec2DIterator {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.width * self.height {
            return None;
        }

        let res = Some((
            self.current_index % self.width,
            self.current_index / self.width,
        ));

        // debug!("{}=>{res:?}", self.current_index);
        self.current_index += 1;
        res
    }
}

#[test]
fn integrity() {
    let mut vec = Vec2D::new_from_element(0, 10, 10);

    vec.elems[1] = 1;
    vec.elems[vec.height] = 2;

    assert_eq!(vec.get(1, 0), Some(&1));
    assert_eq!(vec.get(0, 1), Some(&2));
}

#[test]
fn iterator() {
    let vec = Vec2D::new_from_vec(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], 3, 4).unwrap();

    let mut iterator = vec.iter();

    assert_eq!(
        iterator,
        Vec2DIterator {
            current_index: 0,
            width: 3,
            height: 4
        }
    );

    assert_eq!(iterator.next(), Some((0, 0)));
    assert_eq!(iterator.next(), Some((1, 0)));
    assert_eq!(iterator.next(), Some((2, 0)));
    assert_eq!(iterator.next(), Some((0, 1)));
    assert_eq!(iterator.next(), Some((1, 1)));
    assert_eq!(iterator.next(), Some((2, 1)));
    assert_eq!(iterator.next(), Some((0, 2)));
    assert_eq!(iterator.next(), Some((1, 2)));
    assert_eq!(iterator.next(), Some((2, 2)));
    assert_eq!(iterator.next(), Some((0, 3)));
    assert_eq!(iterator.next(), Some((1, 3)));
    assert_eq!(iterator.next(), Some((2, 3)));
    assert_eq!(iterator.next(), None);
}
